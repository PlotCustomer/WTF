use std::{env, fs, io, path::Path};

use axum_server::tls_rustls::RustlsConfig;
use thiserror::Error;

use crate::config::Config;

#[derive(Debug, Error)]
pub enum TlsError {
    #[error("could not prepare the TLS certificate: {0}")]
    Io(#[from] io::Error),
    #[error("could not generate the TLS certificate: {0}")]
    Generate(#[from] rcgen::Error),
}

pub fn install_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("the ring crypto provider is available");
    }
}

pub async fn config(config: &Config) -> Result<RustlsConfig, TlsError> {
    ensure_certificate(&config.tls_cert_path, &config.tls_key_path)?;

    RustlsConfig::from_pem_file(&config.tls_cert_path, &config.tls_key_path)
        .await
        .map_err(TlsError::Io)
}

fn ensure_certificate(cert_path: &Path, key_path: &Path) -> Result<(), TlsError> {
    if cert_path.is_file() && key_path.is_file() {
        return Ok(());
    }

    if let Some(parent) = cert_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let names = subject_alt_names();
    let certified = rcgen::generate_simple_self_signed(names)?;

    fs::write(cert_path, certified.cert.pem())?;
    fs::write(key_path, certified.signing_key.serialize_pem())?;
    restrict_key_permissions(key_path)?;

    tracing::info!(
        cert = %cert_path.display(),
        "generated a self-signed TLS certificate"
    );

    Ok(())
}

fn subject_alt_names() -> Vec<String> {
    let mut names = vec![
        "localhost".to_owned(),
        "127.0.0.1".to_owned(),
        "::1".to_owned(),
    ];

    if let Ok(hostname) = env::var("HOSTNAME")
        && !hostname.trim().is_empty()
    {
        names.push(hostname);
    }

    if let Ok(extra) = env::var("TLS_SUBJECT_ALT_NAMES") {
        names.extend(
            extra
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned),
        );
    }

    names.sort();
    names.dedup();
    names
}

#[cfg(unix)]
fn restrict_key_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_key_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_alt_names_are_unique_and_include_localhost() {
        let names = subject_alt_names();
        assert!(names.contains(&"localhost".to_owned()));
        let mut unique = names.clone();
        unique.dedup();
        assert_eq!(names, unique);
    }
}
