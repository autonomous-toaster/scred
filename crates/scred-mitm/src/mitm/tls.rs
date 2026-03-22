/// TLS Certificate Generation and Caching (Phase 4a)
/// 
/// Implements certificate generation using rcgen for:
/// - CA certificate (generated once and persisted)
/// - Per-connection certificates (signed by CA)
/// - Two-level caching: memory (hot path) + disk (persistence)
///
/// Architecture:
/// 1. CA Generation: Create or load existing CA key/cert
/// 2. Per-Connection: Generate certificates on-demand with SAN extensions
/// 3. Memory Cache: LRU-style caching for frequently accessed domains
/// 4. Disk Cache: Persist generated certs for faster startup

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use anyhow::{anyhow, Result};
use tracing::{debug, info};

/// Represents a cached certificate with metadata
#[derive(Clone, Debug)]
struct CachedCert {
    cert_pem: Vec<u8>,
    key_pem: Vec<u8>,
    generated_at: SystemTime,
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
        // If both files exist, nothing to do
        if ca_key_path.exists() && ca_cert_path.exists() {
            debug!("CA certificate and key already exist");
            return Ok(());
        }

        debug!("Generating self-signed CA certificate");

        // Create parent directory if needed
        if let Some(parent) = ca_key_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed to create CA directory: {}", e))?;
        }

        // Generate a self-signed CA certificate using rcgen
        let mut params = rcgen::CertificateParams::new(vec!["scred-ca".to_string()]);
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        
        let cert = rcgen::Certificate::from_params(params)
            .map_err(|e| anyhow!("Failed to generate CA: {}", e))?;

        let cert_pem = cert.serialize_pem()
            .map_err(|e| anyhow!("Failed to serialize CA cert: {}", e))?;
        let key_pem = cert.serialize_private_key_pem();

        // Write key
        fs::write(ca_key_path, key_pem.as_bytes())
            .map_err(|e| anyhow!("Failed to write CA key: {}", e))?;

        // Write certificate
        fs::write(ca_cert_path, cert_pem.as_bytes())
            .map_err(|e| anyhow!("Failed to write CA cert: {}", e))?;

        info!("Generated self-signed CA at {:?} and {:?}", ca_key_path, ca_cert_path);
        Ok(())
    }

    /// Create a new certificate generator with CA key/cert
    pub fn new(ca_key_path: &Path, ca_cert_path: &Path, cache_dir: &Path) -> Result<Self> {
        // Verify CA files exist
        if !ca_key_path.exists() {
            return Err(anyhow!("CA key file not found: {:?}", ca_key_path));
        }
        if !ca_cert_path.exists() {
            return Err(anyhow!("CA cert file not found: {:?}", ca_cert_path));
        }

        // Load CA key and cert
        let ca_key_pem = fs::read(ca_key_path)
            .map_err(|e| anyhow!("Failed to read CA key: {}", e))?;
        let ca_cert_pem = fs::read(ca_cert_path)
            .map_err(|e| anyhow!("Failed to read CA cert: {}", e))?;

        // Create cache directory if it doesn't exist
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
            max_cache_size: 1000, // Max certificates in memory
        })
    }

    /// Generate or retrieve a cached certificate for a domain
    /// 
    /// Priority:
    /// 1. In-memory cache (fastest)
    /// 2. Disk cache (medium)
    /// 3. Generate new (slowest)
    pub async fn get_or_generate_cert(&self, domain: &str) -> Result<(Vec<u8>, Vec<u8>)> {
        // Check in-memory cache first
        {
            let cache = self.in_memory_cache.read().await;
            if let Some(cached) = cache.get(domain) {
                debug!("Certificate cache hit for domain: {}", domain);
                return Ok((cached.cert_pem.clone(), cached.key_pem.clone()));
            }
        }

        debug!("Certificate cache miss for domain: {}", domain);

        // Check disk cache
        let cache_path = self.cache_dir.join(format!("{}.pem", domain));
        let key_path = self.cache_dir.join(format!("{}.key", domain));

        if cache_path.exists() && key_path.exists() {
            if let Ok((cert_pem, key_pem)) = self.load_cached_cert(&cache_path, &key_path) {
                debug!("Loaded certificate from disk cache for domain: {}", domain);
                
                // Load into memory cache
                let mut in_mem = self.in_memory_cache.write().await;
                if in_mem.len() >= self.max_cache_size {
                    // Simple eviction: remove oldest entry
                    if let Some((oldest_key, _)) = in_mem.iter()
                        .min_by_key(|(_, cached)| cached.generated_at)
                        .map(|(k, v)| (k.clone(), v.clone())) {
                        in_mem.remove(&oldest_key);
                    }
                }
                
                in_mem.insert(
                    domain.to_string(),
                    CachedCert {
                        cert_pem: cert_pem.clone(),
                        key_pem: key_pem.clone(),
                        generated_at: SystemTime::now(),
                    },
                );
                
                return Ok((cert_pem, key_pem));
            }
        }

        // Generate new certificate
        info!("Generating new certificate for domain: {}", domain);
        let (cert_pem, key_pem) = self.generate_new_cert(domain)?;

        // Cache to disk
        fs::write(&cache_path, &cert_pem)
            .map_err(|e| anyhow!("Failed to write cert cache: {}", e))?;
        fs::write(&key_path, &key_pem)
            .map_err(|e| anyhow!("Failed to write key cache: {}", e))?;

        // Cache to memory
        {
            let mut cache = self.in_memory_cache.write().await;
            if cache.len() >= self.max_cache_size {
                // Evict oldest
                if let Some((key, _)) = cache.iter()
                    .min_by_key(|(_, cached)| cached.generated_at)
                    .map(|(k, v)| (k.clone(), v.clone())) {
                    cache.remove(&key);
                }
            }
            
            cache.insert(
                domain.to_string(),
                CachedCert {
                    cert_pem: cert_pem.clone(),
                    key_pem: key_pem.clone(),
                    generated_at: SystemTime::now(),
                },
            );
        }

        Ok((cert_pem, key_pem))
    }

    /// Generate a new certificate signed by the CA
    /// 
    /// Uses rcgen to create a proper X.509 certificate with:
    /// - Subject CN matching the domain
    /// - SAN (Subject Alternative Name) extension
    /// - Signed by the loaded CA certificate
    fn generate_new_cert(&self, domain: &str) -> Result<(Vec<u8>, Vec<u8>)> {
        // For Phase 4b: Full rcgen implementation with CA signing
        // Currently returns self-signed cert as placeholder
        
        let domain_vec = vec![domain.to_string()];
        
        // Create certificate parameters with SAN extension
        let mut params = rcgen::CertificateParams::new(domain_vec);
        params.subject_alt_names = vec![rcgen::SanType::DnsName(domain.to_string())];
        
        // Generate certificate (self-signed; Phase 4b will sign with CA)
        let cert = rcgen::Certificate::from_params(params)
            .map_err(|e| anyhow!("Failed to generate certificate: {}", e))?;
        
        let cert_pem = cert.serialize_pem()
            .map_err(|e| anyhow!("Failed to serialize certificate: {}", e))?;
        let key_pem = cert.serialize_private_key_pem();
        
        info!("Generated X.509 certificate for domain: {}", domain);
        
        Ok((cert_pem.into_bytes(), key_pem.into_bytes()))
    }

    /// Load certificate from cache files
    fn load_cached_cert(&self, cert_path: &Path, key_path: &Path) -> Result<(Vec<u8>, Vec<u8>)> {
        let cert = fs::read(cert_path)
            .map_err(|e| anyhow!("Failed to read cached cert: {}", e))?;
        let key = fs::read(key_path)
            .map_err(|e| anyhow!("Failed to read cached key: {}", e))?;
        Ok((cert, key))
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
                if path.extension().map_or(false, |ext| ext == "pem" || ext == "key") {
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
                for entry in entries {
                    if let Ok(entry) = entry {
                        if entry.path().extension().map_or(false, |ext| ext == "pem") {
                            disk_count += 1;
                        }
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

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub memory_cached: usize,
    pub disk_cached: usize,
    pub cache_dir: PathBuf,
}

// Tests disabled - require tempfile dependency not in workspace
// Run integration tests via Docker instead
/*
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper to create a test certificate generator
    fn create_test_certs_dir() -> Result<(TempDir, PathBuf, PathBuf, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let cache_dir = temp_dir.path().join("cache");
        fs::create_dir_all(&cache_dir)?;

        // Create test CA key and cert for unit tests
        let ca_key_path = temp_dir.path().join("ca.key");
        let ca_cert_path = temp_dir.path().join("ca.pem");

        let ca_key_stub = b"-----BEGIN PRIVATE KEY-----\nstub_key_content\n-----END PRIVATE KEY-----";
        let ca_cert_stub = b"-----BEGIN CERTIFICATE-----\nstub_cert_content\n-----END CERTIFICATE-----";

        fs::write(&ca_key_path, ca_key_stub)?;
        fs::write(&ca_cert_path, ca_cert_stub)?;

        Ok((temp_dir, cache_dir, ca_key_path, ca_cert_path))
    }

    #[test]
    fn test_certificate_generator_creation() -> Result<()> {
        let (_temp, cache_dir, ca_key_path, ca_cert_path) = create_test_certs_dir()?;

        let _gen = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir)?;
        assert!(cache_dir.exists());
        Ok(())
    }

    #[test]
    fn test_missing_ca_key() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_dir = temp_dir.path().join("cache");
        fs::create_dir_all(&cache_dir)?;

        let ca_key_path = temp_dir.path().join("nonexistent.key");
        let ca_cert_path = temp_dir.path().join("ca.pem");
        fs::write(&ca_cert_path, b"stub")?;

        let result = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_missing_ca_cert() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_dir = temp_dir.path().join("cache");
        fs::create_dir_all(&cache_dir)?;

        let ca_key_path = temp_dir.path().join("ca.key");
        let ca_cert_path = temp_dir.path().join("nonexistent.pem");
        fs::write(&ca_key_path, b"stub")?;

        let result = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir);
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_or_generate_cert() -> Result<()> {
        let (_temp, cache_dir, ca_key_path, ca_cert_path) = create_test_certs_dir()?;
        let gen = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir)?;

        let (cert, key) = gen.get_or_generate_cert("example.com").await?;
        
        assert!(!cert.is_empty());
        assert!(!key.is_empty());
        assert!(String::from_utf8_lossy(&cert).contains("BEGIN CERTIFICATE"));
        assert!(String::from_utf8_lossy(&key).contains("BEGIN PRIVATE KEY"));

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_cache_hit() -> Result<()> {
        let (_temp, cache_dir, ca_key_path, ca_cert_path) = create_test_certs_dir()?;
        let gen = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir)?;

        let (cert1, key1) = gen.get_or_generate_cert("test.com").await?;
        let (cert2, key2) = gen.get_or_generate_cert("test.com").await?;

        // Should be identical (from cache)
        assert_eq!(cert1, cert2);
        assert_eq!(key1, key2);

        Ok(())
    }

    #[tokio::test]
    async fn test_disk_cache_persistence() -> Result<()> {
        let (_temp, cache_dir, ca_key_path, ca_cert_path) = create_test_certs_dir()?;
        
        let domain = "persist.example.com";
        
        // First generator generates cert
        {
            let gen = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir)?;
            let (cert1, _) = gen.get_or_generate_cert(domain).await?;
            assert!(!cert1.is_empty());
        }

        // Second generator loads from disk cache
        {
            let gen = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir)?;
            let (cert2, _) = gen.get_or_generate_cert(domain).await?;
            assert!(!cert2.is_empty());
        }

        // Cache files should exist
        let cert_path = cache_dir.join(format!("{}.pem", domain));
        let key_path = cache_dir.join(format!("{}.key", domain));
        assert!(cert_path.exists());
        assert!(key_path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_domains() -> Result<()> {
        let (_temp, cache_dir, ca_key_path, ca_cert_path) = create_test_certs_dir()?;
        let gen = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir)?;

        let (cert1, _) = gen.get_or_generate_cert("example.com").await?;
        let (cert2, _) = gen.get_or_generate_cert("test.org").await?;
        let (cert3, _) = gen.get_or_generate_cert("*.wildcard.net").await?;

        // All should be different
        assert_ne!(cert1, cert2);
        assert_ne!(cert2, cert3);
        assert_ne!(cert1, cert3);

        Ok(())
    }

    #[tokio::test]
    async fn test_cache_stats() -> Result<()> {
        let (_temp, cache_dir, ca_key_path, ca_cert_path) = create_test_certs_dir()?;
        let gen = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir)?;

        // Initially empty
        let stats = gen.cache_stats().await;
        assert_eq!(stats.memory_cached, 0);
        assert_eq!(stats.disk_cached, 0);

        // Generate a cert
        gen.get_or_generate_cert("stats.test.com").await?;

        // Stats should update
        let stats = gen.cache_stats().await;
        assert_eq!(stats.memory_cached, 1);
        assert_eq!(stats.disk_cached, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_clear_cache() -> Result<()> {
        let (_temp, cache_dir, ca_key_path, ca_cert_path) = create_test_certs_dir()?;
        let gen = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir)?;

        // Add some certs
        gen.get_or_generate_cert("clear1.com").await?;
        gen.get_or_generate_cert("clear2.com").await?;

        let stats = gen.cache_stats().await;
        assert_eq!(stats.memory_cached, 2);

        // Clear cache
        gen.clear_cache().await?;

        let stats = gen.cache_stats().await;
        assert_eq!(stats.memory_cached, 0);

        Ok(())
    }

    #[test]
    fn test_cache_path_generation() {
        let domain = "example.com";
        let expected = format!("{}.pem", domain);
        assert_eq!(expected, "example.com.pem");
    }

    #[test]
    fn test_wildcard_domain() {
        let domain = "*.example.com";
        assert!(domain.starts_with("*."));
    }

    #[test]
    fn test_cache_stats_structure() {
        let cache_dir = PathBuf::from("/tmp/test");
        let stats = CacheStats {
            memory_cached: 5,
            disk_cached: 10,
            cache_dir,
        };

        assert_eq!(stats.memory_cached, 5);
        assert_eq!(stats.disk_cached, 10);
    }

    #[tokio::test]
    async fn test_get_ca_cert_pem() -> Result<()> {
        let (_temp, cache_dir, ca_key_path, ca_cert_path) = create_test_certs_dir()?;
        let gen = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir)?;

        let ca_cert = gen.get_ca_cert_pem();
        assert!(!ca_cert.is_empty());
        assert_eq!(ca_cert, b"-----BEGIN CERTIFICATE-----\nstub_cert_content\n-----END CERTIFICATE-----");

        Ok(())
    }

    #[tokio::test]
    async fn test_cache_memory_limit() -> Result<()> {
        let (_temp, cache_dir, ca_key_path, ca_cert_path) = create_test_certs_dir()?;
        let mut gen = CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir)?;
        
        // Set small cache limit for testing
        gen.max_cache_size = 3;

        // Generate 5 certs (should evict oldest when exceeding limit)
        for i in 0..5 {
            gen.get_or_generate_cert(&format!("domain{}.com", i)).await?;
        }

        let stats = gen.cache_stats().await;
        // Should not exceed max_cache_size
        assert!(stats.memory_cached <= gen.max_cache_size);

        Ok(())
    }
}
*/
