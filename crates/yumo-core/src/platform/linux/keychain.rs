use crate::error::{AppError, AppResult};
use crate::platform::traits::PlatformKeychain;

pub struct LinuxKeychain;

impl PlatformKeychain for LinuxKeychain {
    fn store_key(_service: &str, _account: &str, _password: &str) -> AppResult<()> {
        Err(AppError::Recording("Linux credential storage not yet implemented".into()))
    }

    fn get_key(_service: &str, _account: &str) -> AppResult<Option<String>> {
        Err(AppError::Recording("Linux credential storage not yet implemented".into()))
    }

    fn delete_key(_service: &str, _account: &str) -> AppResult<()> {
        Err(AppError::Recording("Linux credential storage not yet implemented".into()))
    }
}

pub fn store_key(service: &str, account: &str, password: &str) -> AppResult<()> {
    LinuxKeychain::store_key(service, account, password)
}

pub fn get_key(service: &str, account: &str) -> AppResult<Option<String>> {
    LinuxKeychain::get_key(service, account)
}

pub fn delete_key(service: &str, account: &str) -> AppResult<()> {
    LinuxKeychain::delete_key(service, account)
}
