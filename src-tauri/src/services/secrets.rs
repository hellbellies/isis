//! Secrets (API tokens) in the OS keychain: macOS Keychain, Windows Credential
//! Manager, Secret Service on Linux. Secrets never cross IPC back to the frontend.

use crate::error::AppResult;

/// Storage for a single secret. Abstracted so tests don't touch the real keychain.
pub trait SecretStore {
    fn get(&self) -> AppResult<Option<String>>;
    fn set(&self, secret: &str) -> AppResult<()>;
    fn delete(&self) -> AppResult<()>;
}

/// OS keychain entry (macOS Keychain, Windows Credential Manager, Secret Service on Linux).
pub struct KeyringStore {
    service: &'static str,
    user: &'static str,
}

impl KeyringStore {
    pub const fn new(service: &'static str, user: &'static str) -> Self {
        Self { service, user }
    }

    fn entry(&self) -> AppResult<keyring::Entry> {
        Ok(keyring::Entry::new(self.service, self.user)?)
    }
}

impl SecretStore for KeyringStore {
    fn get(&self) -> AppResult<Option<String>> {
        match self.entry()?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn set(&self, secret: &str) -> AppResult<()> {
        Ok(self.entry()?.set_password(secret)?)
    }

    fn delete(&self) -> AppResult<()> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;

    use super::*;

    /// In-memory secret for tests, so they never touch the real keychain.
    #[derive(Default)]
    pub struct MemoryStore(RefCell<Option<String>>);

    impl SecretStore for MemoryStore {
        fn get(&self) -> AppResult<Option<String>> {
            Ok(self.0.borrow().clone())
        }
        fn set(&self, secret: &str) -> AppResult<()> {
            *self.0.borrow_mut() = Some(secret.to_owned());
            Ok(())
        }
        fn delete(&self) -> AppResult<()> {
            *self.0.borrow_mut() = None;
            Ok(())
        }
    }
}
