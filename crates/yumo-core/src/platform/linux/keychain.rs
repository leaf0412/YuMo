use keyring::Entry;

use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformKeychain;

// ---------------------------------------------------------------------------
// LinuxKeychain — PlatformKeychain implementation
//
// Uses the `keyring` crate which backends to Secret Service (libsecret /
// KWallet) on Linux via the `sync-secret-service` feature.
// ---------------------------------------------------------------------------

pub struct LinuxKeychain;

impl PlatformKeychain for LinuxKeychain {
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

pub fn store_key(service: &str, account: &str, password: &str) -> AppResult<()> {
    LinuxKeychain::store_key(service, account, password)
}

pub fn get_key(service: &str, account: &str) -> AppResult<Option<String>> {
    LinuxKeychain::get_key(service, account)
}

pub fn delete_key(service: &str, account: &str) -> AppResult<()> {
    LinuxKeychain::delete_key(service, account)
}

// ---------------------------------------------------------------------------
// Internal implementation
// ---------------------------------------------------------------------------

fn store_key_impl(service: &str, account: &str, password: &str) -> AppResult<()> {
    log::info!("[keychain] store_key service={} account={}", service, account);
    let entry = Entry::new(service, account)
        .map_err(|e| AppError::Io(format!("keychain entry creation failed: {}", e)))?;
    entry
        .set_password(password)
        .map_err(|e| {
            log::error!("[keychain] store failed account={}: {}", account, e);
            AppError::Io(format!("keychain store failed: {}", e))
        })
}

fn get_key_impl(service: &str, account: &str) -> AppResult<Option<String>> {
    log::info!("[keychain] get_key service={} account={}", service, account);
    let entry = Entry::new(service, account)
        .map_err(|e| AppError::Io(format!("keychain entry creation failed: {}", e)))?;
    match entry.get_password() {
        Ok(password) => {
            log::info!("[keychain] get_key account={} => found", account);
            Ok(Some(password))
        }
        Err(keyring::Error::NoEntry) => {
            log::info!("[keychain] no key found for account={}", account);
            Ok(None)
        }
        Err(e) => {
            log::error!("[keychain] read failed account={}: {}", account, e);
            Err(AppError::Io(format!("keychain read failed: {}", e)))
        }
    }
}

fn delete_key_impl(service: &str, account: &str) -> AppResult<()> {
    log::info!("[keychain] delete_key service={} account={}", service, account);
    let entry = Entry::new(service, account)
        .map_err(|e| AppError::Io(format!("keychain entry creation failed: {}", e)))?;
    match entry.delete_credential() {
        Ok(()) => {
            log::info!("[keychain] key deleted for account={}", account);
            Ok(())
        }
        Err(keyring::Error::NoEntry) => {
            log::info!("[keychain] no key to delete for account={}", account);
            Ok(())
        }
        Err(e) => {
            log::error!("[keychain] delete failed account={}: {}", account, e);
            Err(AppError::Io(format!("keychain delete failed: {}", e)))
        }
    }
}
