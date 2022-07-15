use aes_gcm_siv::{
    aead::{Aead, NewAead},
    Aes256GcmSiv, Key, Nonce,
};
use anyhow::anyhow;
use chrono::Utc;
use rand::{thread_rng, RngCore};
use sea_orm::entity::*;
use sha3::{Digest, Sha3_256};

use moonramp_core::{aes_gcm_siv, anyhow, chrono, rand, sea_orm, sha3, Hash};
use moonramp_entity::{cipher::Cipher, key_encryption_key};

use crate::KeyCustodian;

pub struct MasterKeyEncryptionKeyCustodian {
    master_key_encryption_key: Vec<u8>,
    master_key_encryption_key_id: Hash,
    cipher: Cipher,
}

impl MasterKeyEncryptionKeyCustodian {
    pub fn new(master_key_encryption_key: Vec<u8>) -> anyhow::Result<Self> {
        let mut hasher = Sha3_256::new();
        hasher.update(&master_key_encryption_key);
        let master_key_encryption_key_id = Hash::try_from(hasher.finalize().to_vec())?;

        Ok(MasterKeyEncryptionKeyCustodian {
            master_key_encryption_key,
            master_key_encryption_key_id,
            cipher: Cipher::Aes256GcmSiv,
        })
    }

    pub fn id(&self) -> Hash {
        self.master_key_encryption_key_id.clone()
    }
}

impl KeyCustodian for MasterKeyEncryptionKeyCustodian {
    type Secret = [u8; 32];
    type LockedKey = key_encryption_key::Model;
    type ActiveLockedKey = key_encryption_key::ActiveModel;

    fn gen_secret(&self) -> anyhow::Result<[u8; 32]> {
        let mut secret = [0u8; 32];
        thread_rng().fill_bytes(&mut secret);
        Ok(secret)
    }

    fn lock(&self, secret: Self::Secret) -> anyhow::Result<Self::ActiveLockedKey> {
        let key = Key::from_slice(&self.master_key_encryption_key);
        let cipher = Aes256GcmSiv::new(key);

        let mut n_raw = vec![0u8; 12];
        thread_rng().fill_bytes(&mut n_raw);
        let nonce = Nonce::from_slice(&n_raw);

        let mut hasher = Sha3_256::new();
        hasher.update(&secret);
        let id = Hash::try_from(hasher.finalize().to_vec())?;

        let encrypted_key_encryption_key = cipher
            .encrypt(nonce, secret.as_ref())
            .map_err(|err| anyhow!("{}", err))?;

        Ok(key_encryption_key::ActiveModel {
            id: Set(id),
            master_key_encryption_key_id: Set(self.master_key_encryption_key_id.clone()),
            cipher: Set(self.cipher.clone()),
            key: Set(encrypted_key_encryption_key),
            nonce: Set(n_raw),
            created_at: Set(Utc::now()),
        })
    }

    fn unlock(&self, locked_key: Self::LockedKey) -> anyhow::Result<Self::Secret> {
        let key = Key::from_slice(&self.master_key_encryption_key);
        let cipher = Aes256GcmSiv::new(key);
        let nonce = Nonce::from_slice(&locked_key.nonce);
        Ok(cipher
            .decrypt(nonce, locked_key.key.as_ref())
            .map_err(|err| anyhow!("{}", err))?
            .try_into()
            .map_err(|_| anyhow!("Invalid KeyEncryptionKey"))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_key_encryption_key_custodian() {
        let secret = b"an example very very secret key.";
        let mkek = MasterKeyEncryptionKeyCustodian::new(secret.to_vec())
            .expect("Invalid MasterKeyEncryptionKeyCustodian");
        assert_eq!(
            mkek.id().to_string(),
            "Fgm9dLoNoRgdUwEWB5QLFHdhccYY2Zx5egCrY4gnqJpf".to_string()
        );
        let secret = b"BTH a very very very secret key.";
        let kek = mkek.lock(*secret);
        assert!(kek.is_ok());
        let kek = kek.expect("Invalid KeyEncryptionKey");
        let kek = key_encryption_key::Model {
            id: kek.id.unwrap(),
            master_key_encryption_key_id: kek.master_key_encryption_key_id.unwrap(),
            cipher: kek.cipher.unwrap(),
            key: kek.key.unwrap(),
            nonce: kek.nonce.unwrap(),
            created_at: kek.created_at.unwrap(),
        };
        let unlocked_secret = mkek.unlock(kek);
        assert_eq!(unlocked_secret.ok(), Some(*secret,));
    }
}
