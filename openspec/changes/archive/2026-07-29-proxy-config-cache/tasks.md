## 1. Cache proxy config in get_proxy_url

- [x] 1.1 Add `OnceLock` cache in `get_proxy_url` (`crates/scred-http/src/upstream_connection.rs`):
  - Add `use std::sync::OnceLock;`
  - Create a `static RESOLVER: OnceLock<MitmConfig>` inside the function
  - Use `RESOLVER.get_or_init(|| MitmConfig::from_env())` instead of calling `from_env()` directly
