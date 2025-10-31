use task_queue_rs::config::{Config, StorageBackend};

#[test]
fn test_default_config() {
    let config = Config::default();
    assert!(config.worker_count > 0);
    assert_eq!(config.max_queue_size, 10000);
    assert_eq!(config.storage_backend, StorageBackend::Memory);
}

#[test]
fn test_config_validation() {
    let mut config = Config::default();
    assert!(config.validate().is_ok());

    config.worker_count = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_custom_config() {
    let config = Config::new(8);
    assert_eq!(config.worker_count, 8);
}
