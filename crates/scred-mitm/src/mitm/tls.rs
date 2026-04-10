//! TLS Certificate Generation and Caching
//!
//! Implements certificate generation using rcgen for:
//! - CA certificate (generated once and persisted)
//! - Per-connection certificates (signed by CA)
//! - Two-level caching: memory (hot path) + disk (persistence)
//! - Automatic expiry detection and regeneration
//!
//! Uses ECDSA P-256 with proper validity periods for LibreSSL compatibility.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use time::{Duration, OffsetDateTime};
use ::pem;
use x509_parser::parse_x509_certificate;

// Certificate validity: CA lives 10 years, domain certs 1 year
const CA_VALIDITY_YEARS: i64 = 10;
const DOMAIN_VALIDITY_DAYS: i64 = 365;

/// Check if a PEM-encoded certificate is still valid (not expired)
/// Returns true if the certificate is valid, false if expired or unparseable
fn is_cert_valid(cert_pem: &[u8]) -> bool {
    // Parse PEM to extract the DER-encoded certificate
    let pem = match ::pem::parse(cert_pem) {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to parse PEM: {}", e);
            return false;
        }
    };

    // Parse X.509 certificate
    let cert = match parse_x509_certificate(&pem.contents()) {
        Ok((_, cert)) => cert,
        Err(e) => {
            warn!("Failed to parse X.509 certificate: {}", e);
            return false;
        }
    };

    // Check validity period using ASN1Time
    let now = x509_parser::time::ASN1Time::now();
    cert.validity().is_valid_at(now)
}

/// Extract the expiration time from a PEM-encoded certificate
fn get_cert_expiry(cert_pem: &[u8]) -> Option<OffsetDateTime> {
    let pem = ::pem::parse(cert_pem).ok()?;
    let (_, cert) = parse_x509_certificate(&pem.contents()).ok()?;
    // to_datetime returns OffsetDateTime directly
    Some(cert.validity().not_after.to_datetime())
}

/// Represents a cached certificate with metadata
#[derive(Clone, Debug)]
struct CachedCert {
    cert_pem: Vec<u8>,
    key_pem: Vec<u8>,
    generated_at: SystemTime,
    expires_at: Option<OffsetDateTime>, // Parsed expiry time for quick checks
}

/// Certificate generator with CA support and caching
#[derive(Clone)]
pub struct CertificateGenerator {
    ca_key_pem: Vec<u8>,
    ca_cert_pem: Vec<u8>,
    cache_dir: PathBuf,
    in_memory_cache: Arc<RwLock<HashMap<String, CachedCert>>>,
    max_cache_size: usize,
}

impl CertificateGenerator {
    /// Generate a self-signed CA certificate and key if they don't exist
    pub fn generate_ca_if_missing(ca_key_path: &Path, ca_cert_path: &Path) -> Result<()> {
        if ca_key_path.exists() && ca_cert_path.exists() {
            debug!("CA certificate and key already exist");
            return Ok(());
        }

        debug!("Generating self-signed CA certificate");

        if let Some(parent) = ca_key_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed to create CA directory: {}", e))?;
        }

        // Use ECDSA P-256 - widely supported including LibreSSL
        let alg = &rcgen::PKCS_ECDSA_P256_SHA256;
        
        // Generate CA with proper attributes
        let mut params = rcgen::CertificateParams::new(vec![]);
        
        // Set proper distinguished name
        params.distinguished_name = rcgen::DistinguishedName::new();
        params.distinguished_name.push(rcgen::DnType::CommonName, "scred-mitm-ca");
        params.distinguished_name.push(rcgen::DnType::OrganizationName, "SCRED");
        
        // Mark as CA with no constraints
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        
        // Set key usage for CA (keyCertSign and cRLSign)
        params.key_usages = vec![
            rcgen::KeyUsagePurpose::KeyCertSign,
            rcgen::KeyUsagePurpose::CrlSign,
        ];
        
        // Use ECDSA P-256 algorithm
        params.alg = alg;
        
        // Set validity period (10 years, starting 24h ago for clock skew)
        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::days(1);
        params.not_after = now + Duration::days(CA_VALIDITY_YEARS * 365);

        // Generate key pair (ECDSA P-256 is supported by rcgen)
        let key_pair = rcgen::KeyPair::generate(alg)
            .map_err(|e| anyhow!("Failed to generate ECDSA key pair: {}", e))?;
        params.key_pair = Some(key_pair);

        let cert = rcgen::Certificate::from_params(params)
            .map_err(|e| anyhow!("Failed to generate CA: {}", e))?;

        let cert_pem = cert.serialize_pem()
            .map_err(|e| anyhow!("Failed to serialize CA cert: {}", e))?;
        let key_pem = cert.serialize_private_key_pem();

        fs::write(ca_key_path, key_pem.as_bytes())
            .map_err(|e| anyhow!("Failed to write CA key: {}", e))?;
        fs::write(ca_cert_path, cert_pem.as_bytes())
            .map_err(|e| anyhow!("Failed to write CA cert: {}", e))?;

        info!("Generated ECDSA P-256 CA certificate at {:?} and {:?}", ca_key_path, ca_cert_path);
        Ok(())
    }

    /// Create a new certificate generator with CA key/cert
    pub fn new(ca_key_path: &Path, ca_cert_path: &Path, cache_dir: &Path) -> Result<Self> {
        if !ca_key_path.exists() {
            return Err(anyhow!("CA key file not found: {:?}", ca_key_path));
        }
        if !ca_cert_path.exists() {
            return Err(anyhow!("CA cert file not found: {:?}", ca_cert_path));
        }

        let ca_key_pem = fs::read(ca_key_path)
            .map_err(|e| anyhow!("Failed to read CA key: {}", e))?;
        let ca_cert_pem = fs::read(ca_cert_path)
            .map_err(|e| anyhow!("Failed to read CA cert: {}", e))?;

        if !cache_dir.exists() {
            fs::create_dir_all(cache_dir)
                .map_err(|e| anyhow!("Failed to create cache dir: {}", e))?;
        }

        info!("Certificate generator initialized with CA from {:?}", ca_key_path);

        Ok(Self {
            ca_key_pem,
            ca_cert_pem,
            cache_dir: cache_dir.to_path_buf(),
            in_memory_cache: Arc::new(RwLock::new(HashMap::new())),
            max_cache_size: 1000,
        })
    }

    /// Generate or retrieve a cached certificate for a domain
    pub async fn get_or_generate_cert(&self, domain: &str) -> Result<(Vec<u8>, Vec<u8>)> {
        // Check in-memory cache (async, no blocking)
        {
            let cache = self.in_memory_cache.read().await;
            if let Some(cached) = cache.get(domain) {
                // Check if certificate is still valid
                if let Some(expires_at) = cached.expires_at {
                    let now = OffsetDateTime::now_utc();
                    if now < expires_at {
                        debug!("Certificate cache hit for domain: {}", domain);
                        return Ok((cached.cert_pem.clone(), cached.key_pem.clone()));
                    } else {
                        debug!("Cached certificate expired for domain: {} (expired at {:?})", domain, expires_at);
                        // Don't return here - will regenerate below
                    }
                } else {
                    // No expiry info parsed, validate with is_cert_valid
                    if is_cert_valid(&cached.cert_pem) {
                        debug!("Certificate cache hit for domain: {}", domain);
                        return Ok((cached.cert_pem.clone(), cached.key_pem.clone()));
                    } else {
                        debug!("Cached certificate invalid for domain: {}", domain);
                    }
                }
            }
        }

        debug!("Certificate cache miss for domain: {}", domain);

        // Move blocking operations to spawn_blocking
        let cache_dir = self.cache_dir.clone();
        let ca_key_pem = self.ca_key_pem.clone();
        let ca_cert_pem = self.ca_cert_pem.clone();
        let domain_owned = domain.to_string();

        let result = tokio::task::spawn_blocking(move || {
            // All blocking I/O and CPU-heavy operations here
            let cache_path = cache_dir.join(format!("{}.pem", domain_owned));
            let key_path = cache_dir.join(format!("{}.key", domain_owned));

            // Check disk cache with expiry validation
            if cache_path.exists() && key_path.exists() {
                if let (Ok(cert), Ok(key)) = (fs::read(&cache_path), fs::read(&key_path)) {
                    // Validate certificate before using
                    if is_cert_valid(&cert) {
                        return Ok::<_, anyhow::Error>((cert, key, true)); // true = from cache
                    } else {
                        debug!("Disk cached certificate expired for domain: {}", domain_owned);
                        // Delete expired files to clean up
                        let _ = fs::remove_file(&cache_path);
                        let _ = fs::remove_file(&key_path);
                    }
                }
            }

            // Generate new certificate (CPU-heavy + uses loaded CA)
            let (cert, key) = generate_cert_signed_by_ca(&domain_owned, &ca_key_pem, &ca_cert_pem)?;
            Ok((cert, key, false)) // false = newly generated
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking error: {}", e))??;

        let (cert_pem, key_pem, from_cache) = result;

        // Only write to disk if newly generated
        if !from_cache {
            let cache_path = self.cache_dir.join(format!("{}.pem", domain));
            let key_path = self.cache_dir.join(format!("{}.key", domain));
            
            tokio::fs::write(&cache_path, &cert_pem).await
                .map_err(|e| anyhow!("Failed to write cert cache: {}", e))?;
            tokio::fs::write(&key_path, &key_pem).await
                .map_err(|e| anyhow!("Failed to write key cache: {}", e))?;
        }

        // Cache to memory
        {
            let mut cache = self.in_memory_cache.write().await;
            if cache.len() >= self.max_cache_size {
                if let Some((key, _)) = cache
                    .iter()
                    .min_by_key(|(_, cached)| cached.generated_at)
                    .map(|(k, v)| (k.clone(), v.clone()))
                {
                    cache.remove(&key);
                }
            }
            cache.insert(
                domain.to_string(),
                CachedCert {
                    cert_pem: cert_pem.clone(),
                    key_pem: key_pem.clone(),
                    generated_at: SystemTime::now(),
                    expires_at: get_cert_expiry(&cert_pem),
                },
            );
        }

        if !from_cache {
            info!("Generated CA-signed ECDSA P-256 certificate for domain: {}", domain);
        }
        Ok((cert_pem, key_pem))
    }

    /// Get CA certificate PEM
    pub fn get_ca_cert_pem(&self) -> Vec<u8> {
        self.ca_cert_pem.clone()
    }

    /// Clear all cached certificates
    pub async fn clear_cache(&self) -> Result<()> {
        // Clear disk cache
        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "pem" || ext == "key") {
                    fs::remove_file(path)?;
                }
            }
        }

        // Clear memory cache
        let mut cache = self.in_memory_cache.write().await;
        cache.clear();

        info!("Certificate cache cleared");
        Ok(())
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let in_mem = self.in_memory_cache.read().await;

        let mut disk_count = 0;
        if self.cache_dir.exists() {
            if let Ok(entries) = fs::read_dir(&self.cache_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().is_some_and(|ext| ext == "pem") {
                        disk_count += 1;
                    }
                }
            }
        }

        CacheStats {
            memory_cached: in_mem.len(),
            disk_cached: disk_count,
            cache_dir: self.cache_dir.clone(),
        }
    }
}

/// Generate a certificate signed by CA (blocking operation)
/// Uses ECDSA P-256 for LibreSSL compatibility
fn generate_cert_signed_by_ca(domain: &str, ca_key_pem: &[u8], ca_cert_pem: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let ca_key_str = String::from_utf8_lossy(ca_key_pem);
    let ca_cert_str = String::from_utf8_lossy(ca_cert_pem);

    // Use ECDSA P-256 algorithm
    let alg = &rcgen::PKCS_ECDSA_P256_SHA256;

    // Create KeyPair from CA private key
    let ca_keypair = rcgen::KeyPair::from_pem(&ca_key_str)
        .map_err(|e| anyhow!("Failed to parse CA key: {}", e))?;

    // Parse CA certificate
    let ca_params = rcgen::CertificateParams::from_ca_cert_pem(&ca_cert_str, ca_keypair)
        .map_err(|e| anyhow!("Failed to parse CA cert: {}", e))?;

    let ca_cert = rcgen::Certificate::from_params(ca_params)
        .map_err(|e| anyhow!("Failed to create CA cert object: {}", e))?;

    // Create domain certificate
    let mut params = rcgen::CertificateParams::new(vec![domain.to_string()]);
    
    // Set distinguished name
    params.distinguished_name = rcgen::DistinguishedName::new();
    params.distinguished_name.push(rcgen::DnType::CommonName, domain);
    
    // Set SAN extension
    params.subject_alt_names = vec![rcgen::SanType::DnsName(domain.to_string())];
    
    // Not a CA
    params.is_ca = rcgen::IsCa::NoCa;
    
    // Key usage for server certificate
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::DigitalSignature,
        rcgen::KeyUsagePurpose::KeyEncipherment,
    ];
    
    // Extended key usage for TLS server
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
    
    // Use ECDSA P-256 (same as CA)
    params.alg = alg;
    
    // Set validity period (1 year, starting 24h ago for clock skew)
    let now = OffsetDateTime::now_utc();
    params.not_before = now - Duration::days(1);
    params.not_after = now + Duration::days(DOMAIN_VALIDITY_DAYS);
    
    // Generate ECDSA key pair for domain cert
    let domain_key_pair = rcgen::KeyPair::generate(alg)
        .map_err(|e| anyhow!("Failed to generate domain ECDSA key pair: {}", e))?;
    params.key_pair = Some(domain_key_pair);

    // Generate domain certificate
    let domain_cert = rcgen::Certificate::from_params(params)
        .map_err(|e| anyhow!("Failed to generate domain certificate: {}", e))?;
    
    // Sign with CA
    let cert_pem = domain_cert
        .serialize_pem_with_signer(&ca_cert)
        .map_err(|e| anyhow!("Failed to sign certificate: {}", e))?;
    
    let key_pem = domain_cert.serialize_private_key_pem();

    Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub memory_cached: usize,
    pub disk_cached: usize,
    pub cache_dir: PathBuf,
}
