use crate::error::AppError;
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

/// Store (create or update) an API key in the macOS Keychain.
pub fn store_key(service: &str, account: &str, password: &str) -> Result<(), AppError> {
    set_generic_password(service, account, password.as_bytes())
        .map_err(|e| AppError::Io(format!("Keychain store failed: {}", e)))
}

/// Retrieve an API key from the macOS Keychain.
/// Returns `Ok(None)` when the entry does not exist.
pub fn get_key(service: &str, account: &str) -> Result<Option<String>, AppError> {
    match get_generic_password(service, account) {
        Ok(bytes) => {
            let s = String::from_utf8(bytes)
                .map_err(|e| AppError::Io(format!("Keychain decode failed: {}", e)))?;
            Ok(Some(s))
        }
        Err(e) => {
            // errSecItemNotFound = -25300
            if e.code() == -25300 {
                Ok(None)
            } else {
                Err(AppError::Io(format!("Keychain read failed: {}", e)))
            }
        }
    }
}

/// Delete an API key from the macOS Keychain.
/// Returns `Ok(())` even when the entry does not exist.
pub fn delete_key(service: &str, account: &str) -> Result<(), AppError> {
    match delete_generic_password(service, account) {
        Ok(()) => Ok(()),
        Err(e) => {
            // errSecItemNotFound = -25300
            if e.code() == -25300 {
                Ok(())
            } else {
                Err(AppError::Io(format!("Keychain delete failed: {}", e)))
            }
        }
    }
}
