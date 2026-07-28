use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scred_mitm::mitm::tls::CertificateGenerator;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn setup_ca(temp_dir: &PathBuf) -> (PathBuf, PathBuf, PathBuf) {
    let ca_key_path = temp_dir.join("ca-key.pem");
    let ca_cert_path = temp_dir.join("ca-cert.pem");
    let cache_dir = temp_dir.join("cache");

    std::fs::create_dir_all(temp_dir).unwrap();
    CertificateGenerator::generate_ca_if_missing(&ca_key_path, &ca_cert_path).unwrap();

    (ca_key_path, ca_cert_path, cache_dir)
}

fn benchmark_cert_generation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("mitm_cert_generation");
    group.sample_size(10);

    // Benchmark: cache miss (first generation)
    {
        let temp_dir = std::env::temp_dir().join("scred-bench-cert-miss");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let (ca_key_path, ca_cert_path, cache_dir) = setup_ca(&temp_dir);
        let gen = Arc::new(
            CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir).unwrap(),
        );

        group.bench_function("cache_miss", |b| {
            b.to_async(&rt).iter(|| {
                let gen = gen.clone();
                let domain = format!("bench-{}.example.com", rand_domain_suffix());
                async move {
                    let result = gen.get_or_generate_cert(&domain).await.unwrap();
                    black_box(result);
                }
            });
        });

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // Benchmark: cache hit (second call with same domain)
    {
        let temp_dir = std::env::temp_dir().join("scred-bench-cert-hit");
        let _ = std::fs::remove_dir_all(&temp_dir);
        let (ca_key_path, ca_cert_path, cache_dir) = setup_ca(&temp_dir);
        let gen = Arc::new(
            CertificateGenerator::new(&ca_key_path, &ca_cert_path, &cache_dir).unwrap(),
        );

        // Prime the cache
        rt.block_on(async {
            gen.get_or_generate_cert("cached.example.com")
                .await
                .unwrap();
        });

        group.bench_function("cache_hit", |b| {
            b.to_async(&rt).iter(|| {
                let gen = gen.clone();
                async move {
                    let result = gen
                        .get_or_generate_cert("cached.example.com")
                        .await
                        .unwrap();
                    black_box(result);
                }
            });
        });

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    group.finish();
}

fn rand_domain_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", nanos)
}

criterion_group!(benches, benchmark_cert_generation);
criterion_main!(benches);
