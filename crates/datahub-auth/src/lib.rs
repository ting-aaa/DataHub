use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

const MINIMUM_PASSWORD_LENGTH: usize = 12;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("password must contain at least {MINIMUM_PASSWORD_LENGTH} characters")]
    PasswordTooShort,
    #[error("password hashing failed")]
    PasswordHash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedToken {
    pub plaintext: String,
    pub digest: String,
}

/// Hashes a password with Argon2id and a cryptographically random salt.
///
/// # Errors
///
/// Returns [`AuthError::PasswordTooShort`] for passwords below the local policy,
/// or [`AuthError::PasswordHash`] if the selected hashing backend fails.
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    if password.chars().count() < MINIMUM_PASSWORD_LENGTH {
        return Err(AuthError::PasswordTooShort);
    }
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthError::PasswordHash)
}

#[must_use]
pub fn verify_password(password: &str, encoded_hash: &str) -> bool {
    PasswordHash::new(encoded_hash).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}

#[must_use]
pub fn issue_token() -> IssuedToken {
    let plaintext = format!("{}.{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
    let digest = digest_token(&plaintext);
    IssuedToken { plaintext, digest }
}

#[must_use]
pub fn digest_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{AuthError, digest_token, hash_password, issue_token, verify_password};

    #[test]
    fn password_hash_verifies_only_the_original_password() {
        let hash = hash_password("correct horse battery staple").expect("valid password");
        assert!(verify_password("correct horse battery staple", &hash));
        assert!(!verify_password("incorrect horse battery staple", &hash));
    }

    #[test]
    fn rejects_short_passwords() {
        assert!(matches!(
            hash_password("too-short"),
            Err(AuthError::PasswordTooShort)
        ));
    }

    #[test]
    fn issued_token_digest_is_stable_and_does_not_expose_plaintext() {
        let issued = issue_token();
        assert_eq!(issued.digest, digest_token(&issued.plaintext));
        assert!(!issued.digest.contains(&issued.plaintext));
    }
}
