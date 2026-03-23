//! Integration tests for SCRED configuration system

use scred_config::ConfigLoader;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_loader_with_explicit_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");
    
    let yaml_content = r#"
scred-proxy:
  listen:
    port: 9999
  upstream:
    url: "https://backend.example.com"
"#;
    
    fs::write(&config_path, yaml_content).unwrap();
    
    // Use load_from_file with explicit path
    let config = ConfigLoader::load_from_file(&config_path).unwrap();
    
    assert!(config.scred_proxy.is_some());
    let proxy_cfg = config.scred_proxy.unwrap();
    assert_eq!(proxy_cfg.listen.port, Some(9999));
    assert_eq!(proxy_cfg.upstream.url, Some("https://backend.example.com".to_string()));
}

#[test]
fn test_config_validation() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");
    
    let yaml_content = r#"
scred-proxy:
  listen:
    port: 9999
  upstream:
    url: "https://backend.example.com"
"#;
    
    fs::write(&config_path, yaml_content).unwrap();
    
    let config = ConfigLoader::load_from_file(&config_path).unwrap();
    let result = ConfigLoader::validate(&config);
    
    assert!(result.is_ok());
}

#[test]
fn test_config_with_per_path_rules() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");
    
    let yaml_content = r#"
scred-proxy:
  listen:
    port: 9999
  upstream:
    url: "https://backend.example.com"
  rules:
    - path: "/health"
      redact: false
    - path: "/api/internal/*"
      redact: false
"#;
    
    fs::write(&config_path, yaml_content).unwrap();
    
    let config = ConfigLoader::load_from_file(&config_path).unwrap();
    
    assert!(config.scred_proxy.is_some());
    let proxy_cfg = config.scred_proxy.unwrap();
    assert_eq!(proxy_cfg.rules.len(), 2);
    assert_eq!(proxy_cfg.rules[0].path, "/health");
    assert!(!proxy_cfg.rules[0].redact);
}

#[test]
fn test_all_three_sections_in_single_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");
    
    let yaml_content = r#"
scred-cli:
  mode: "auto"
  streaming: true

scred-proxy:
  listen:
    port: 9999
  upstream:
    url: "https://backend.example.com"

scred-mitm:
  listen:
    port: 8080
"#;
    
    fs::write(&config_path, yaml_content).unwrap();
    
    let config = ConfigLoader::load_from_file(&config_path).unwrap();
    
    assert!(config.scred_cli.is_some());
    assert!(config.scred_proxy.is_some());
    assert!(config.scred_mitm.is_some());
}

