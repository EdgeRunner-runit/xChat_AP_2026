use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Mode unlocked by user authentication.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum VaultMode {
    Real,
    Decoy,
    Invalid,
}

/// Duress and Decoy PIN authenticator.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DuressVaultManager {
    primary_pin_hash: [u8; 32],
    decoy_pin_hash: [u8; 32],
}

impl DuressVaultManager {
    pub fn new(primary_pin: &str, decoy_pin: &str) -> Self {
        assert_ne!(
            primary_pin, decoy_pin,
            "Primary and Decoy PIN must be different"
        );
        Self {
            primary_pin_hash: Self::hash_pin(primary_pin),
            decoy_pin_hash: Self::hash_pin(decoy_pin),
        }
    }

    fn hash_pin(pin: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"xchat-pin-salt-v1");
        hasher.update(pin.as_bytes());
        hasher.finalize().into()
    }

    /// Authenticate a user-provided PIN.
    pub fn authenticate(&self, input_pin: &str) -> VaultMode {
        let input_hash = Self::hash_pin(input_pin);
        if input_hash == self.primary_pin_hash {
            VaultMode::Real
        } else if input_hash == self.decoy_pin_hash {
            VaultMode::Decoy
        } else {
            VaultMode::Invalid
        }
    }

    /// Derive local storage encryption key for whichever mode was authenticated.
    pub fn derive_storage_key(&self, input_pin: &str, mode: VaultMode) -> Option<[u8; 32]> {
        if mode == VaultMode::Invalid {
            return None;
        }

        let mut hasher = Sha256::new();
        hasher.update(input_pin.as_bytes());
        match mode {
            VaultMode::Real => hasher.update(b"xchat-real-storage-key"),
            VaultMode::Decoy => hasher.update(b"xchat-decoy-storage-key"),
            VaultMode::Invalid => unreachable!(),
        }
        Some(hasher.finalize().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duress_pin_authentication() {
        let vault = DuressVaultManager::new("123456", "999999");

        assert_eq!(vault.authenticate("123456"), VaultMode::Real);
        assert_eq!(vault.authenticate("999999"), VaultMode::Decoy);
        assert_eq!(vault.authenticate("000000"), VaultMode::Invalid);

        let real_key = vault.derive_storage_key("123456", VaultMode::Real).unwrap();
        let decoy_key = vault
            .derive_storage_key("999999", VaultMode::Decoy)
            .unwrap();
        assert_ne!(real_key, decoy_key);
    }
}
