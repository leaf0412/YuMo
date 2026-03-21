use crate::error::{AppError, AppResult};
use crate::mask;
use crate::platform::traits::PlatformKeychain;
use keyring::Entry;
use log::{error, info};

// ---------------------------------------------------------------------------
// WindowsKeychain — PlatformKeychain implementation
// ---------------------------------------------------------------------------

pub struct WindowsKeychain;

impl PlatformKeychain for WindowsKeychain {
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
    WindowsKeychain::store_key(service, account, password)
}

pub fn get_key(service: &str, account: &str) -> AppResult<Option<String>> {
    WindowsKeychain::get_key(service, account)
}

pub fn delete_key(service: &str, account: &str) -> AppResult<()> {
    WindowsKeychain::delete_key(service, account)
}

// ---------------------------------------------------------------------------
// Internal implementation
// ---------------------------------------------------------------------------

fn store_key_impl(service: &str, account: &str, password: &str) -> AppResult<()> {
    info!(
        "[keychain] store_key account={} value={}",
        account,
        mask::mask(password)
    );
    let entry = Entry::new(service, account).map_err(|e| {
        error!("[keychain] failed to create entry for account={}: {}", account, e);
        AppError::Io(format!("Keychain entry creation failed: {}", e))
    })?;
    entry.set_password(password).map_err(|e| {
        error!("[keychain] store failed for account={}: {}", account, e);
        AppError::Io(format!("Keychain store failed: {}", e))
    })
}

fn get_key_impl(service: &str, account: &str) -> AppResult<Option<String>> {
    info!("[keychain] getting key for account={}", account);
    let entry = Entry::new(service, account).map_err(|e| {
        error!("[keychain] failed to create entry for account={}: {}", account, e);
        AppError::Io(format!("Keychain entry creation failed: {}", e))
    })?;
    match entry.get_password() {
        Ok(pw) => {
            info!(
                "[keychain] get_key account={} => {}",
                account,
                mask::mask(&pw)
            );
            Ok(Some(pw))
        }
        Err(keyring::Error::NoEntry) => {
            info!("[keychain] no key found for account={}", account);
            Ok(None)
        }
        Err(e) => {
            error!("[keychain] read failed for account={}: {}", account, e);
            Err(AppError::Io(format!("Keychain read failed: {}", e)))
        }
    }
}

fn delete_key_impl(service: &str, account: &str) -> AppResult<()> {
    info!("[keychain] deleting key for account={}", account);
    let entry = Entry::new(service, account).map_err(|e| {
        error!("[keychain] failed to create entry for account={}: {}", account, e);
        AppError::Io(format!("Keychain entry creation failed: {}", e))
    })?;
    match entry.delete_credential() {
        Ok(_) => {
            info!("[keychain] key deleted for account={}", account);
            Ok(())
        }
        Err(keyring::Error::NoEntry) => {
            info!("[keychain] no key to delete for account={}", account);
            Ok(())
        }
        Err(e) => {
            error!("[keychain] delete failed for account={}: {}", account, e);
            Err(AppError::Io(format!("Keychain delete failed: {}", e)))
        }
    }
}
