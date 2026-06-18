use crate::internal::auth::error::{AuthError, AuthResult};

/// Production password rules for fintech.
pub fn validate_password(password: &str) -> AuthResult<()> {
    if password.len() < 12 {
        return Err(AuthError::Validation(
            "password must be at least 12 characters".into(),
        ));
    }
    if password.len() > 128 {
        return Err(AuthError::Validation(
            "password must be at most 128 characters".into(),
        ));
    }

    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password
        .chars()
        .any(|c| !c.is_ascii_alphanumeric() && !c.is_whitespace());

    if !(has_lower && has_upper && has_digit && has_special) {
        return Err(AuthError::Validation(
            "password must include uppercase, lowercase, digit, and special character".into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_weak_password() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("alllowercase123!").is_err());
    }

    #[test]
    fn accepts_strong_password() {
        assert!(validate_password("Str0ng!Passw0rd").is_ok());
    }
}
