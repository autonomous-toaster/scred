use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;
use tokio::io::{AsyncWriteExt, BufReader, duplex};
use tokio::runtime::Runtime;

fn benchmark_forward_simple(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("proxy_forward_simple");
    group.sample_size(10);

    for body_size in [1024, 102400, 1048576].iter() {
        let body = vec![b'A'; *body_size];
        let request = build_http_request(*body_size, &body);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}kb", body_size / 1024)),
            body_size,
            |b, _| {
                b.to_async(&rt).iter(|| {
                    let req = request.clone();
                    async move {
                        let (mut client_write, client_read) = duplex(65536);
                        let (mut upstream_write, mut upstream_read) = duplex(65536);

                        client_write.write_all(&req).await.unwrap();
                        drop(client_write);

                        let mut client_buf_reader = BufReader::new(client_read);

                        scred_proxy::handler::forward_simple(
                            &mut client_buf_reader,
                            &mut upstream_write,
                            "GET /path HTTP/1.1",
                        )
                        .await
                        .unwrap();

                        drop(upstream_write);
                        let mut upstream_data = Vec::new();
                        tokio::io::AsyncReadExt::read_to_end(&mut upstream_read, &mut upstream_data)
                            .await
                            .unwrap();
                        black_box(upstream_data);
                    }
                });
            },
        );
    }

    group.finish();
}

fn benchmark_forward_with_policy(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("proxy_forward_with_policy");
    group.sample_size(10);

    let engine = Arc::new(
        scred_policy::PolicyEngine::new(
            scred_config::PolicyConfig {
                enabled: false,
                providers: vec![],
                ..Default::default()
            },
        )
        .unwrap(),
    );

    for body_size in [1024, 102400, 1048576].iter() {
        let body = vec![b'A'; *body_size];
        let request = build_http_request(*body_size, &body);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}kb", body_size / 1024)),
            body_size,
            |b, _| {
                let engine = engine.clone();
                let req = request.clone();
                b.to_async(&rt).iter(|| {
                    let engine = engine.clone();
                    let req = req.clone();
                    async move {
                        let (mut client_write, client_read) = duplex(65536);
                        let (mut upstream_write, mut upstream_read) = duplex(65536);

                        client_write.write_all(&req).await.unwrap();
                        drop(client_write);

                        let mut client_buf_reader = BufReader::new(client_read);

                        scred_proxy::handler::forward_with_policy(
                            &mut client_buf_reader,
                            &mut upstream_write,
                            "GET /path HTTP/1.1",
                            &engine,
                            "example.com",
                        )
                        .await
                        .unwrap();

                        drop(upstream_write);
                        let mut upstream_data = Vec::new();
                        tokio::io::AsyncReadExt::read_to_end(&mut upstream_read, &mut upstream_data)
                            .await
                            .unwrap();
                        black_box(upstream_data);
                    }
                });
            },
        );
    }

    group.finish();
}

fn build_http_request(body_size: usize, body: &[u8]) -> Vec<u8> {
    let mut req = Vec::new();
    req.extend_from_slice(b"Host: example.com\r\n");
    req.extend_from_slice(b"Content-Type: text/plain\r\n");
    req.extend_from_slice(format!("Content-Length: {}\r\n", body_size).as_bytes());
    req.extend_from_slice(b"\r\n");
    req.extend_from_slice(body);
    req
}

criterion_group!(benches, benchmark_forward_simple, benchmark_forward_with_policy);
criterion_main!(benches);
