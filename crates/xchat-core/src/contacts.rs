use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};
use xchat_crypto::x3dh::PrekeyBundle;

use crate::errors::CoreError;

/// Individual contact entry saved in the user's private address book.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Contact {
    pub alias: String,
    pub address: String,
    pub bundle: PrekeyBundle,
    pub added_at: u64,
    pub last_seen: Option<u64>,
}

impl Contact {
    pub fn new(alias: String, bundle: PrekeyBundle, added_at: u64) -> Self {
        let address = bundle.identity_key.address();
        Self {
            alias,
            address,
            bundle,
            added_at,
            last_seen: None,
        }
    }

    pub fn fingerprint(&self) -> String {
        self.bundle.identity_key.fingerprint()
    }
}

/// Contact book managing known peers, address aliases, and prekey bundles.
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct ContactBook {
    contacts_by_alias: HashMap<String, Contact>,
    contacts_by_address: HashMap<String, String>, // Address -> Alias
}

impl ContactBook {
    pub fn new() -> Self {
        Self {
            contacts_by_alias: HashMap::new(),
            contacts_by_address: HashMap::new(),
        }
    }

    /// Add or update a contact with an alias and their public prekey bundle.
    pub fn add_contact(&mut self, alias: &str, bundle: PrekeyBundle, now: u64) {
        let contact = Contact::new(alias.to_string(), bundle, now);
        let address = contact.address.clone();
        self.contacts_by_address.insert(address, alias.to_string());
        self.contacts_by_alias.insert(alias.to_string(), contact);
    }

    /// Retrieve contact by user alias or address.
    pub fn get(&self, alias_or_addr: &str) -> Option<&Contact> {
        if let Some(c) = self.contacts_by_alias.get(alias_or_addr) {
            Some(c)
        } else if let Some(alias) = self.contacts_by_address.get(alias_or_addr) {
            self.contacts_by_alias.get(alias)
        } else {
            None
        }
    }

    /// Remove a contact.
    pub fn remove(&mut self, alias_or_addr: &str) -> Option<Contact> {
        if let Some(c) = self.contacts_by_alias.remove(alias_or_addr) {
            self.contacts_by_address.remove(&c.address);
            Some(c)
        } else if let Some(alias) = self.contacts_by_address.remove(alias_or_addr) {
            self.contacts_by_alias.remove(&alias)
        } else {
            None
        }
    }

    /// Record when a peer was last seen active.
    pub fn update_last_seen(&mut self, alias_or_addr: &str, now: u64) {
        let target_alias = if let Some(alias) = self.contacts_by_address.get(alias_or_addr) {
            Some(alias.clone())
        } else if self.contacts_by_alias.contains_key(alias_or_addr) {
            Some(alias_or_addr.to_string())
        } else {
            None
        };

        if let Some(alias) = target_alias {
            if let Some(contact) = self.contacts_by_alias.get_mut(&alias) {
                contact.last_seen = Some(now);
            }
        }
    }

    /// List all contacts sorted alphabetically by alias.
    pub fn list(&self) -> Vec<&Contact> {
        let mut list: Vec<&Contact> = self.contacts_by_alias.values().collect();
        list.sort_by(|a, b| a.alias.cmp(&b.alias));
        list
    }

    pub fn len(&self) -> usize {
        self.contacts_by_alias.len()
    }

    pub fn is_empty(&self) -> bool {
        self.contacts_by_alias.is_empty()
    }

    /// Save contact book to disk.
    pub fn save_to_disk(&self, path: &Path) -> Result<(), CoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| CoreError::StorageError(e.to_string()))?;
        }
        let data = bincode::serialize(self).map_err(|e| CoreError::StorageError(e.to_string()))?;
        let mut file = File::create(path).map_err(|e| CoreError::StorageError(e.to_string()))?;
        file.write_all(&data)
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        Ok(())
    }

    /// Load contact book from disk.
    pub fn load_from_disk(path: &Path) -> Result<Self, CoreError> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let mut file = File::open(path).map_err(|e| CoreError::StorageError(e.to_string()))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| CoreError::StorageError(e.to_string()))?;
        bincode::deserialize(&buffer).map_err(|e| CoreError::StorageError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use xchat_crypto::{identity::IdentityKeyPair, x3dh::SignedPrekeyPair};

    #[test]
    fn test_contact_book_crud_and_persistence() {
        let mut book = ContactBook::new();

        let alice_identity = IdentityKeyPair::generate();
        let alice_spk = SignedPrekeyPair::generate(&alice_identity);
        let alice_bundle = PrekeyBundle {
            identity_key: alice_identity.public_key(),
            signed_prekey: alice_spk.pubkey,
            signed_prekey_signature: alice_spk.signature,
            one_time_prekey: None,
        };

        let now = 1000;
        book.add_contact("Alice", alice_bundle.clone(), now);
        assert_eq!(book.len(), 1);

        let retrieved = book.get("Alice").expect("Alice exists");
        assert_eq!(retrieved.alias, "Alice");
        assert_eq!(retrieved.address, alice_bundle.identity_key.address());

        // Also retrievable by address
        let by_addr = book
            .get(&alice_bundle.identity_key.address())
            .expect("by address");
        assert_eq!(by_addr.alias, "Alice");

        // Update last seen
        book.update_last_seen("Alice", 1050);
        assert_eq!(book.get("Alice").unwrap().last_seen, Some(1050));

        // Test persistence
        let temp_file = NamedTempFile::new().unwrap();
        book.save_to_disk(temp_file.path()).unwrap();

        let loaded = ContactBook::load_from_disk(temp_file.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.get("Alice").unwrap().last_seen, Some(1050));
    }
}
