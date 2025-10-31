use std::env;
use std::fs;
use std::sync::Mutex;
use task_queue_rs::config::{Config, StorageBackend};

// Mutex to ensure environment variable tests don't run in parallel
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_load_config_from_yaml() {
    // Create a temporary YAML config file with unique name
    let yaml_content = r#"
worker_count: 16
max_queue_size: 20000
task_timeout_secs: 500
max_retries: 10
storage_backend: "memory"
enable_persistence: true
shutdown_timeout_secs: 45
"#;

    let filename = "test_yaml_1.yaml";
    fs::write(filename, yaml_content).unwrap();

    let config = Config::from_file("test_yaml_1").unwrap();

    assert_eq!(config.worker_count, 16);
    assert_eq!(config.max_queue_size, 20000);
    assert_eq!(config.task_timeout_secs, 500);
    assert_eq!(config.max_retries, 10);
    assert_eq!(config.storage_backend, StorageBackend::Memory);
    assert!(config.enable_persistence);
    assert_eq!(config.shutdown_timeout_secs, 45);

    // Cleanup
    fs::remove_file(filename).unwrap();
}

#[test]
fn test_load_config_from_toml() {
    // Create a temporary TOML config file with unique name
    let toml_content = r#"
worker_count = 12
max_queue_size = 15000
task_timeout_secs = 400
max_retries = 7
storage_backend = "redis"
enable_persistence = false
shutdown_timeout_secs = 50
"#;

    let filename = "test_toml_2.toml";
    fs::write(filename, toml_content).unwrap();

    let config = Config::from_file("test_toml_2").unwrap();

    assert_eq!(config.worker_count, 12);
    assert_eq!(config.max_queue_size, 15000);
    assert_eq!(config.task_timeout_secs, 400);
    assert_eq!(config.max_retries, 7);
    assert_eq!(config.storage_backend, StorageBackend::Redis);
    assert!(!config.enable_persistence);
    assert_eq!(config.shutdown_timeout_secs, 50);

    // Cleanup
    fs::remove_file(filename).unwrap();
}

#[test]
fn test_load_config_from_env() {
    let _lock = ENV_MUTEX.lock().unwrap();

    env::set_var("TASK_QUEUE_WORKER_COUNT", "20");
    env::set_var("TASK_QUEUE_MAX_QUEUE_SIZE", "30000");
    env::set_var("TASK_QUEUE_TASK_TIMEOUT_SECS", "700");
    env::set_var("TASK_QUEUE_MAX_RETRIES", "15");
    env::set_var("TASK_QUEUE_STORAGE_BACKEND", "postgresql");
    env::set_var("TASK_QUEUE_ENABLE_PERSISTENCE", "true");
    env::set_var("TASK_QUEUE_SHUTDOWN_TIMEOUT_SECS", "90");

    let config = Config::from_env().unwrap();

    assert_eq!(config.worker_count, 20);
    assert_eq!(config.max_queue_size, 30000);
    assert_eq!(config.task_timeout_secs, 700);
    assert_eq!(config.max_retries, 15);
    assert_eq!(config.storage_backend, StorageBackend::PostgreSQL);
    assert!(config.enable_persistence);
    assert_eq!(config.shutdown_timeout_secs, 90);

    // Cleanup
    env::remove_var("TASK_QUEUE_WORKER_COUNT");
    env::remove_var("TASK_QUEUE_MAX_QUEUE_SIZE");
    env::remove_var("TASK_QUEUE_TASK_TIMEOUT_SECS");
    env::remove_var("TASK_QUEUE_MAX_RETRIES");
    env::remove_var("TASK_QUEUE_STORAGE_BACKEND");
    env::remove_var("TASK_QUEUE_ENABLE_PERSISTENCE");
    env::remove_var("TASK_QUEUE_SHUTDOWN_TIMEOUT_SECS");
}

#[test]
fn test_config_validation_fails_for_zero_workers() {
    let yaml_content = r#"
worker_count: 0
max_queue_size: 10000
task_timeout_secs: 300
max_retries: 3
storage_backend: "memory"
enable_persistence: false
shutdown_timeout_secs: 30
"#;

    let filename = "test_invalid_3.yaml";
    fs::write(filename, yaml_content).unwrap();

    let result = Config::from_file("test_invalid_3");
    assert!(result.is_err());

    // Cleanup
    fs::remove_file(filename).unwrap();
}

#[test]
fn test_config_validation_fails_for_zero_queue_size() {
    let yaml_content = r#"
worker_count: 4
max_queue_size: 0
task_timeout_secs: 300
max_retries: 3
storage_backend: "memory"
enable_persistence: false
shutdown_timeout_secs: 30
"#;

    let filename = "test_invalid_4.yaml";
    fs::write(filename, yaml_content).unwrap();

    let result = Config::from_file("test_invalid_4");
    assert!(result.is_err());

    // Cleanup
    fs::remove_file(filename).unwrap();
}

#[test]
fn test_load_falls_back_to_defaults() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // Ensure no env vars are set
    env::remove_var("TASK_QUEUE_CONFIG");
    env::remove_var("TASK_QUEUE_WORKER_COUNT");
    env::remove_var("TASK_QUEUE_MAX_QUEUE_SIZE");
    env::remove_var("TASK_QUEUE_TASK_TIMEOUT_SECS");
    env::remove_var("TASK_QUEUE_MAX_RETRIES");
    env::remove_var("TASK_QUEUE_STORAGE_BACKEND");
    env::remove_var("TASK_QUEUE_ENABLE_PERSISTENCE");
    env::remove_var("TASK_QUEUE_SHUTDOWN_TIMEOUT_SECS");

    let config = Config::load().unwrap();

    // Should use defaults
    assert!(config.worker_count > 0);
    assert_eq!(config.max_queue_size, 10000);
    assert_eq!(config.task_timeout_secs, 300);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.storage_backend, StorageBackend::Memory);
    assert!(!config.enable_persistence);
    assert_eq!(config.shutdown_timeout_secs, 30);
}

#[test]
fn test_env_var_overrides_config_file() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // Create a config file with unique name
    let yaml_content = r#"
worker_count: 8
max_queue_size: 10000
task_timeout_secs: 300
max_retries: 3
storage_backend: "memory"
enable_persistence: false
shutdown_timeout_secs: 30
"#;

    let filename = "test_override_5.yaml";
    fs::write(filename, yaml_content).unwrap();

    // Set environment variable to point to this file
    env::set_var("TASK_QUEUE_CONFIG", "test_override_5");

    let config = Config::load().unwrap();
    assert_eq!(config.worker_count, 8);

    // Cleanup
    env::remove_var("TASK_QUEUE_CONFIG");
    fs::remove_file(filename).unwrap();
}

#[test]
fn test_invalid_storage_backend_env() {
    let _lock = ENV_MUTEX.lock().unwrap();

    env::set_var("TASK_QUEUE_WORKER_COUNT", "4");
    env::set_var("TASK_QUEUE_STORAGE_BACKEND", "invalid_backend");

    let result = Config::from_env();
    assert!(result.is_err());

    // Cleanup
    env::remove_var("TASK_QUEUE_WORKER_COUNT");
    env::remove_var("TASK_QUEUE_STORAGE_BACKEND");
}

#[test]
fn test_partial_env_vars() {
    let _lock = ENV_MUTEX.lock().unwrap();

    // Set only some environment variables
    env::set_var("TASK_QUEUE_WORKER_COUNT", "6");
    env::set_var("TASK_QUEUE_MAX_RETRIES", "8");

    let config = Config::from_env().unwrap();

    // Should have the env var values
    assert_eq!(config.worker_count, 6);
    assert_eq!(config.max_retries, 8);

    // Should have default values for others
    assert_eq!(config.max_queue_size, 10000);
    assert_eq!(config.task_timeout_secs, 300);

    // Cleanup
    env::remove_var("TASK_QUEUE_WORKER_COUNT");
    env::remove_var("TASK_QUEUE_MAX_RETRIES");
}
