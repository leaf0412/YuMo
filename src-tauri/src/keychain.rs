use crate::error::AppError;
use crate::mask;
use log::{error, info};
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

/// Store (create or update) an API key in the macOS Keychain.
pub fn store_key(service: &str, account: &str, password: &str) -> Result<(), AppError> {
    info!("[keychain] store_key account={} value={}", account, mask::mask(password));
    set_generic_password(service, account, password.as_bytes())
        .map_err(|e| {
            error!("[keychain] store failed for account={}: {}", account, e);
            AppError::Io(format!("Keychain store failed: {}", e))
        })
}

/// Retrieve an API key from the macOS Keychain.
/// Returns `Ok(None)` when the entry does not exist.
pub fn get_key(service: &str, account: &str) -> Result<Option<String>, AppError> {
    info!("[keychain] getting key for account={}", account);
    match get_generic_password(service, account) {
        Ok(bytes) => {
            let s = String::from_utf8(bytes)
                .map_err(|e| {
                    error!("[keychain] decode failed for account={}: {}", account, e);
                    AppError::Io(format!("Keychain decode failed: {}", e))
                })?;
            info!("[keychain] get_key account={} => {}", account, mask::mask(&s));
            Ok(Some(s))
        }
        Err(e) => {
            // errSecItemNotFound = -25300
            if e.code() == -25300 {
                info!("[keychain] no key found for account={}", account);
                Ok(None)
            } else {
                error!("[keychain] read failed for account={}: {}", account, e);
                Err(AppError::Io(format!("Keychain read failed: {}", e)))
            }
        }
    }
}

/// Delete an API key from the macOS Keychain.
/// Returns `Ok(())` even when the entry does not exist.
pub fn delete_key(service: &str, account: &str) -> Result<(), AppError> {
    info!("[keychain] deleting key for account={}", account);
    match delete_generic_password(service, account) {
        Ok(()) => {
            info!("[keychain] key deleted for account={}", account);
            Ok(())
        }
        Err(e) => {
            // errSecItemNotFound = -25300
            if e.code() == -25300 {
                info!("[keychain] no key to delete for account={}", account);
                Ok(())
            } else {
                error!("[keychain] delete failed for account={}: {}", account, e);
                Err(AppError::Io(format!("Keychain delete failed: {}", e)))
            }
        }
    }
}
