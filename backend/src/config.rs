use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub auth_enabled: bool,
    pub jwks_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL must be set")?;
        
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8000".to_string())
            .parse::<u16>()
            .context("PORT must be a valid number")?;
        
        let auth_enabled = std::env::var("AUTH_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .context("AUTH_ENABLED must be true or false")?;
        
        let jwks_url = if auth_enabled {
            Some(std::env::var("JWKS_URL")
                .context("JWKS_URL must be set when AUTH_ENABLED is true")?)
        } else {
            None
        };

        Ok(Config {
            database_url,
            port,
            auth_enabled,
            jwks_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::collections::HashMap;

    // Use a mutex to ensure tests don't interfere with each other
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    // Helper struct to save and restore environment variables
    struct EnvGuard {
        saved: HashMap<String, Option<String>>,
    }

    impl EnvGuard {
        fn new() -> Self {
            EnvGuard {
                saved: HashMap::new(),
            }
        }

        fn set(&mut self, key: &str, value: &str) {
            // Save the original value if we haven't already
            if !self.saved.contains_key(key) {
                self.saved.insert(key.to_string(), std::env::var(key).ok());
            }
            // SAFETY: This is only used in single-threaded test contexts with mutex protection
            unsafe {
                std::env::set_var(key, value);
            }
        }

        fn remove(&mut self, key: &str) {
            // Save the original value if we haven't already
            if !self.saved.contains_key(key) {
                self.saved.insert(key.to_string(), std::env::var(key).ok());
            }
            // SAFETY: This is only used in single-threaded test contexts with mutex protection
            unsafe {
                std::env::remove_var(key);
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // Restore all saved environment variables
            for (key, value) in &self.saved {
                // SAFETY: This is only used in single-threaded test contexts with mutex protection
                unsafe {
                    match value {
                        Some(v) => std::env::set_var(key, v),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    #[test]
    fn test_config_from_env_with_defaults() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let mut env = EnvGuard::new();
        
        env.set("DATABASE_URL", "postgresql://test@localhost/test");
        env.remove("PORT");
        env.remove("AUTH_ENABLED");
        env.remove("JWKS_URL");

        let config = Config::from_env().unwrap();
        assert_eq!(config.database_url, "postgresql://test@localhost/test");
        assert_eq!(config.port, 8000);
        assert_eq!(config.auth_enabled, false);
        assert_eq!(config.jwks_url, None);
    }

    #[test]
    fn test_config_from_env_with_auth() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let mut env = EnvGuard::new();
        
        env.set("DATABASE_URL", "postgresql://test@localhost/test");
        env.set("PORT", "3000");
        env.set("AUTH_ENABLED", "true");
        env.set("JWKS_URL", "https://auth.example.com/jwks");

        let config = Config::from_env().unwrap();
        assert_eq!(config.port, 3000);
        assert_eq!(config.auth_enabled, true);
        assert_eq!(config.jwks_url, Some("https://auth.example.com/jwks".to_string()));
    }

    #[test]
    fn test_config_missing_database_url() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let mut env = EnvGuard::new();
        
        env.remove("DATABASE_URL");
        
        let result = Config::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("DATABASE_URL"));
    }

    #[test]
    fn test_config_invalid_port() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let mut env = EnvGuard::new();
        
        env.set("DATABASE_URL", "postgresql://test@localhost/test");
        env.set("PORT", "not_a_number");
        
        let result = Config::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("PORT"));
    }
}