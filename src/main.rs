use regex::Regex;
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::{self, read_dir},
    path::Path,
    process::{exit, Command},
};

fn main() {
    // 1. Parse Keys and values directly from .env file
    let env_vars = load_env_file_keys_values(".env");
    
    // 2. Create env('XXX') key → config section (filename) map from config/*.php
    let config_dir = Path::new("config");
    let env_key_to_section = parse_config_env_keys(config_dir);

    // 3. Load bootstrap/cache/config.php with PHP and convert to JSON
    let config_cache_path = "bootstrap/cache/config.php";
    if !Path::new(config_cache_path).exists() {
        println!("Config cache file not found: {config_cache_path}");
        exit(1);
    }
    
    let config_json = load_config_php_as_json(config_cache_path);
    
    println!("=== Differences between .env and config cache ===");

    let mut found_diff = false;

    // 4. Check differences starting from .env
    for (env_key, env_val) in &env_vars {
        if let Some(section) = env_key_to_section.get(env_key) {
            if let Some(section_val) = config_json.get(section) {
                let section_str = json_value_to_string(section_val);

                let matches = if env_val.to_lowercase() == "null" {
                    // For null values, compare case-insensitively
                    section_str.to_lowercase().contains("null")
                } else {
                    section_str.contains(env_val)
                };

                if !matches {
                    println!("[DIFF] {env_key}");
                    found_diff = true;
                }
            } else {
                println!("[MISSING] Section '{section}' not found in config.php");
                found_diff = true;
            }
        }
        // Ignore keys not in env_key_to_section (no warning)
    }

    if !found_diff {
        println!("No differences found between .env and config cache.");
    }
}

/// Parse keys and values from .env file into HashMap
fn load_env_file_keys_values(path: &str) -> HashMap<String, String> {
    let content = fs::read_to_string(path).expect("Failed to read .env file");
    let mut map = HashMap::new();

    let re = Regex::new(r#"^\s*([A-Z0-9_]+)\s*=\s*(.*)\s*$"#).unwrap();

    for line in content.lines() {
        if let Some(cap) = re.captures(line) {
            let key = cap[1].to_string();
            let mut val = cap[2].trim().to_string();

            // Remove quotes
            if (val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\''))
            {
                val = val[1..val.len() - 1].to_string();
            }

            map.insert(key, val);
        }
    }
    map
}

/// Return env('XXX') call key → config section (filename) map from config/*.php
fn parse_config_env_keys(config_dir: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let re = Regex::new(r#"env\(\s*['"]([A-Z0-9_]+)['"]\s*,?\s*[^)]*\)"#).unwrap();

    for entry in read_dir(config_dir).expect("Failed to read config directory") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("php") {
            let file_stem = path.file_stem().unwrap().to_string_lossy().to_string();
            let content = fs::read_to_string(&path).expect("Failed to read config file");

            for cap in re.captures_iter(&content) {
                let env_key = cap[1].to_string();
                map.entry(env_key).or_insert_with(|| file_stem.clone());
            }
        }
    }

    map
}

/// Load config.php with PHP, convert to JSON and return as Value
fn load_config_php_as_json(path: &str) -> Value {
    let php_code = format!(
        r#"
        function sanitize($data) {{
            if (is_array($data)) {{
                return array_map('sanitize', $data);
            }} elseif (is_object($data)) {{
                return get_class($data);
            }} elseif (is_resource($data)) {{
                return 'resource';
            }}
            return $data;
        }}
        echo json_encode(sanitize(include '{path}'));
    "#
    );

    let output = Command::new("php")
        .arg("-r")
        .arg(&php_code)
        .output()
        .expect("Failed to run php command");

    if !output.status.success() {
        eprintln!("Failed to load {path}");
        std::process::exit(1);
    }

    serde_json::from_slice(&output.stdout).expect("Failed to parse JSON from config.php")
}

/// Convert JSON Value to string
fn json_value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => format!("{val:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, create_dir_all};
    use tempfile::TempDir;

    /// Test case with differences
    #[test]
    fn with_cache_differences() {
        let (env_vars, env_key_to_section, config_json) = setup_test_with_differences();

        let mut found_diff = false;
        for (env_key, env_val) in &env_vars {
            if let Some(section) = env_key_to_section.get(env_key) {
                if let Some(section_val) = config_json.get(section) {
                    let section_str = json_value_to_string(section_val);
                    if !section_str.contains(env_val) {
                        found_diff = true;
                    }
                }
            }
        }

        assert!(found_diff, "Expected to find differences between .env and config cache");
    }

    /// Test case with no differences
    #[test]
    fn no_cache_differences() {
        let (env_vars, env_key_to_section, config_json) = setup_test_no_differences();

        let mut found_diff = false;
        for (env_key, env_val) in &env_vars {
            if let Some(section) = env_key_to_section.get(env_key) {
                if let Some(section_val) = config_json.get(section) {
                    let section_str = json_value_to_string(section_val);
                    if !section_str.contains(env_val) {
                        found_diff = true;
                        break;
                    }
                }
            }
        }

        assert!(!found_diff, "Expected no differences between .env and config cache");
    }

    /// Setup for test with differences
    fn setup_test_with_differences() -> (HashMap<String, String>, HashMap<String, String>, Value) {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        setup_common_files(&temp_path);

        // Create config cache with differences
        let config_cache = r#"<?php
return [
    'database' => [
        'default' => 'mysql',
        'connections' => [
            'mysql' => [
                'host' => '127.0.0.1',  // Different from .env (localhost)
                'port' => '3307',       // Different from .env (3306)
            ],
            'redis' => [
                'default' => [
                    'password' => NULL,
                ],
                'cache' => [
                    'password' => NULL,
                ],
            ],
        ],
    ],
];"#;
        let cache_dir = temp_path.join("bootstrap/cache");
        create_dir_all(&cache_dir).unwrap();
        fs::write(cache_dir.join("config.php"), config_cache).unwrap();

        load_test_data(&temp_path)
    }

    /// Setup for test with no differences
    fn setup_test_no_differences() -> (HashMap<String, String>, HashMap<String, String>, Value) {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        setup_common_files(&temp_path);

        // Create config cache with no differences
        let config_cache = r#"<?php
return [
    'database' => [
        'default' => 'mysql',
        'connections' => [
            'mysql' => [
                'host' => 'localhost',   // Same as .env
                'port' => '3306',        // Same as .env
            ],
            'redis' => [
                'password' => 'null',  // Same as .env
            ],
        ],
    ],
];"#;
        let cache_dir = temp_path.join("bootstrap/cache");
        create_dir_all(&cache_dir).unwrap();
        fs::write(cache_dir.join("config.php"), config_cache).unwrap();

        load_test_data(&temp_path)
    }

    /// Setup common files
    fn setup_common_files(temp_path: &Path) {
        // Create .env file
        let env_content = r#"DB_HOST=localhost
DB_PORT=3306
REDIS_PASSWORD=null"#;

        fs::write(temp_path.join(".env"), env_content).unwrap();

        // Create config/database.php
        let config_dir = temp_path.join("config");
        create_dir_all(&config_dir).unwrap();
        let database_config = r#"<?php
return [
    'default' => env('DB_CONNECTION', 'mysql'),
    'connections' => [
        'mysql' => [
            'host' => env('DB_HOST', '127.0.0.1'),
            'port' => env('DB_PORT', '3306'),
        ],
        'redis' => [
            'password' => env('REDIS_PASSWORD', null),
        ],
    ],
];"#;
        fs::write(config_dir.join("database.php"), database_config).unwrap();
    }

    /// Load test data
    fn load_test_data(temp_path: &Path) -> (HashMap<String, String>, HashMap<String, String>, Value) {
        let env_vars = load_env_file_keys_values(&temp_path.join(".env").to_string_lossy());
        let config_dir = temp_path.join("config");
        let env_key_to_section = parse_config_env_keys(&config_dir);
        let config_json = load_config_php_as_json(&temp_path.join("bootstrap/cache/config.php").to_string_lossy());
        
        (env_vars, env_key_to_section, config_json)
    }
}