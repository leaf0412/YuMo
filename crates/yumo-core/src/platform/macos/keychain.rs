use crate::error::{AppError, AppResult};
use crate::mask;
use crate::platform::traits::PlatformKeychain;
use log::{error, info};
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

// ---------------------------------------------------------------------------
// MacosKeychain — PlatformKeychain implementation
// ---------------------------------------------------------------------------

pub struct MacosKeychain;

impl PlatformKeychain for MacosKeychain {
    fn store_key(service: &str, account: &str, password: &str) -> AppResult<()> {
        store_key_impl(service, account, password)
    }

    fn get_key(service: &str, account: &str) -> AppResult<Option<String>> {
        get_key_impl(service, account)
    }

    fn delete_key(service: &str, account: &str) -> AppResult<()> {
        delete_key_impl(service, account)
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible public functions
// ---------------------------------------------------------------------------

pub fn store_key(service: &str, account: &str, password: &str) -> Result<(), AppError> {
    MacosKeychain::store_key(service, account, password)
}

pub fn get_key(service: &str, account: &str) -> Result<Option<String>, AppError> {
    MacosKeychain::get_key(service, account)
}

pub fn delete_key(service: &str, account: &str) -> Result<(), AppError> {
    MacosKeychain::delete_key(service, account)
}

// ---------------------------------------------------------------------------
// Internal implementation
// ---------------------------------------------------------------------------

fn store_key_impl(service: &str, account: &str, password: &str) -> Result<(), AppError> {
    info!("[keychain] store_key account={} value={}", account, mask::mask(password));
    set_generic_password(service, account, password.as_bytes())
        .map_err(|e| {
            error!("[keychain] store failed for account={}: {}", account, e);
            AppError::Io(format!("Keychain store failed: {}", e))
        })
}

fn get_key_impl(service: &str, account: &str) -> Result<Option<String>, AppError> {
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

fn delete_key_impl(service: &str, account: &str) -> Result<(), AppError> {
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
