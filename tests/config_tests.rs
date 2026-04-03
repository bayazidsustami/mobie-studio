use mobie::config::AppConfig;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_config_roundtrip() {
    let tmp_dir = tempdir().expect("create temp dir");
    let config_file = tmp_dir.path().join("config.toml");

    let mut cfg = AppConfig::default();
    cfg.llm.api_key = "test-key".to_string();
    cfg.llm.base_url = "https://example.com/v1".to_string();
    cfg.llm.model = "test-model".to_string();

    // Mock save_config logic manually since it uses hardcoded path
    let contents = toml::to_string_pretty(&cfg).expect("serialize");
    fs::write(&config_file, contents).expect("write");

    let loaded_contents = fs::read_to_string(&config_file).expect("read");
    let loaded_cfg: AppConfig = toml::from_str(&loaded_contents).expect("deserialize");

    assert_eq!(loaded_cfg.llm.api_key, "test-key");
    assert_eq!(loaded_cfg.llm.base_url, "https://example.com/v1");
    assert_eq!(loaded_cfg.llm.model, "test-model");
}

#[test]
fn test_config_version_default() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.version, 1);
}
