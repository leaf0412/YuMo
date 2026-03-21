use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformKeychain;

pub struct WindowsKeychain;

impl PlatformKeychain for WindowsKeychain {
    fn store_key(_service: &str, _account: &str, _password: &str) -> AppResult<()> {
        Err(AppError::Recording("Windows credential storage not yet implemented".into()))
    }

    fn get_key(_service: &str, _account: &str) -> AppResult<Option<String>> {
        Err(AppError::Recording("Windows credential storage not yet implemented".into()))
    }

    fn delete_key(_service: &str, _account: &str) -> AppResult<()> {
        Err(AppError::Recording("Windows credential storage not yet implemented".into()))
    }
}

pub fn store_key(service: &str, account: &str, password: &str) -> AppResult<()> {
    WindowsKeychain::store_key(service, account, password)
}

pub fn get_key(service: &str, account: &str) -> AppResult<Option<String>> {
    WindowsKeychain::get_key(service, account)
}

pub fn delete_key(service: &str, account: &str) -> AppResult<()> {
    WindowsKeychain::delete_key(service, account)
}
