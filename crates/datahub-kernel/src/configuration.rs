use std::{env, fs, io, path::PathBuf};

use thiserror::Error;

const MAX_SECRET_BYTES: u64 = 64 * 1024;

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("required configuration {0} or {0}_FILE is missing")]
    Missing(String),
    #[error("secret file configured by {name}_FILE could not be read")]
    SecretFile {
        name: String,
        #[source]
        source: io::Error,
    },
    #[error("secret file configured by {0}_FILE exceeds 64 KiB")]
    SecretFileTooLarge(String),
    #[error("required configuration {0} is empty")]
    Empty(String),
}

/// Reads a required value from an external secret file or the environment.
///
/// `{NAME}_FILE` takes precedence over `NAME`; this allows Docker/Kubernetes
/// secrets without placing credentials in process arguments or committed files.
///
/// # Errors
/// Returns a redacted configuration error. Secret values and file contents are
/// never included in the error.
pub fn required_secret(name: &str) -> Result<String, ConfigurationError> {
    secret_value(
        name,
        env::var(format!("{name}_FILE")).ok(),
        env::var(name).ok(),
    )
}

fn secret_value(
    name: &str,
    file: Option<String>,
    direct: Option<String>,
) -> Result<String, ConfigurationError> {
    if let Some(path) = file {
        let path = PathBuf::from(path);
        let metadata = fs::metadata(&path).map_err(|source| ConfigurationError::SecretFile {
            name: name.into(),
            source,
        })?;
        if metadata.len() > MAX_SECRET_BYTES {
            return Err(ConfigurationError::SecretFileTooLarge(name.into()));
        }
        let value = fs::read_to_string(path).map_err(|source| ConfigurationError::SecretFile {
            name: name.into(),
            source,
        })?;
        return non_empty(name, &value);
    }
    direct
        .ok_or_else(|| ConfigurationError::Missing(name.into()))
        .and_then(|value| non_empty(name, &value))
}

fn non_empty(name: &str, value: &str) -> Result<String, ConfigurationError> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        Err(ConfigurationError::Empty(name.into()))
    } else {
        Ok(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{ConfigurationError, MAX_SECRET_BYTES, secret_value};

    fn temporary_file() -> PathBuf {
        std::env::temp_dir().join(format!("datahub-secret-{}.txt", uuid::Uuid::now_v7()))
    }

    #[test]
    fn secret_file_takes_precedence_and_is_trimmed() {
        let path = temporary_file();
        fs::write(&path, "  postgresql://local-only  \n").expect("write secret fixture");
        let value = secret_value(
            "DATABASE_URL",
            Some(path.to_string_lossy().into_owned()),
            Some("ignored".into()),
        )
        .expect("read secret file");
        fs::remove_file(path).expect("remove secret fixture");
        assert_eq!(value, "postgresql://local-only");
    }

    #[test]
    fn missing_empty_and_oversized_values_are_rejected_without_secret_content() {
        assert!(matches!(
            secret_value("DATABASE_URL", None, None),
            Err(ConfigurationError::Missing(_))
        ));
        assert!(matches!(
            secret_value("DATABASE_URL", None, Some("  \n".into())),
            Err(ConfigurationError::Empty(_))
        ));

        let path = temporary_file();
        let marker = "TOP_SECRET_DO_NOT_DISCLOSE";
        let max_secret_bytes = usize::try_from(MAX_SECRET_BYTES).expect("secret limit fits usize");
        let oversized = marker.repeat((max_secret_bytes / marker.len()) + 2);
        fs::write(&path, oversized).expect("write oversized secret fixture");
        let error = secret_value(
            "DATABASE_URL",
            Some(path.to_string_lossy().into_owned()),
            None,
        )
        .expect_err("oversized secret should fail");
        fs::remove_file(path).expect("remove secret fixture");
        assert!(matches!(error, ConfigurationError::SecretFileTooLarge(_)));
        assert!(!error.to_string().contains(marker));
    }

    #[test]
    fn unreadable_secret_error_does_not_disclose_the_configured_path() {
        let path = temporary_file();
        let path_text = path.to_string_lossy().into_owned();
        let error = secret_value("DATABASE_URL", Some(path_text.clone()), None)
            .expect_err("missing secret file should fail");
        assert!(matches!(error, ConfigurationError::SecretFile { .. }));
        assert!(!error.to_string().contains(&path_text));
    }
}
