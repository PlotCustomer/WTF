use std::{env, net::IpAddr, path::PathBuf};

use thiserror::Error;

pub const DEFAULT_HOST: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 3000;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: IpAddr,
    pub port: u16,
    pub database_url: String,
    pub tls_cert_path: PathBuf,
    pub tls_key_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("DATABASE_URL must be set")]
    MissingDatabaseUrl,
    #[error("HOST must be an IP address, got {0}")]
    InvalidHost(String),
    #[error("PORT must be a number between 1 and 65535, got {0}")]
    InvalidPort(String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let host = env::var("HOST").unwrap_or_else(|_| DEFAULT_HOST.to_owned());
        let host = host
            .parse::<IpAddr>()
            .map_err(|_| ConfigError::InvalidHost(host))?;

        let port = env::var("PORT").unwrap_or_else(|_| DEFAULT_PORT.to_string());
        let port = port
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or(ConfigError::InvalidPort(port))?;

        let database_url = env::var("DATABASE_URL").map_err(|_| ConfigError::MissingDatabaseUrl)?;

        let tls_dir = env::temp_dir().join("store-app-tls");
        let tls_cert_path = env_path("TLS_CERT_PATH").unwrap_or_else(|| tls_dir.join("cert.pem"));
        let tls_key_path = env_path("TLS_KEY_PATH").unwrap_or_else(|| tls_dir.join("key.pem"));

        Ok(Self {
            host,
            port,
            database_url,
            tls_cert_path,
            tls_key_path,
        })
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_public_and_on_the_contract_port() {
        assert_eq!(DEFAULT_HOST, "0.0.0.0");
        assert_eq!(DEFAULT_PORT, 3000);
    }
}
