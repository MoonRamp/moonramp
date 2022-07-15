use std::fmt;

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
use moonramp_entity::{cipher::Cipher, encryption_key};

use crate::KeyCustodian;

#[derive(PartialEq, Eq)]
pub struct MerchantScopedSecret {
    pub merchant_id: String,
    pub secret: [u8; 32],
}

impl fmt::Debug for MerchantScopedSecret {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MerchantScopedSecret({})", self.merchant_id)
    }
}

pub struct KeyEncryptionKeyCustodian {
    key_encryption_key: Vec<u8>,
    key_encryption_key_id: Hash,
    cipher: Cipher,
}

impl KeyEncryptionKeyCustodian {
    pub fn new(key_encryption_key: Vec<u8>) -> anyhow::Result<Self> {
        let mut hasher = Sha3_256::new();
        hasher.update(&key_encryption_key);
        let key_encryption_key_id = Hash::try_from(hasher.finalize().to_vec())?;

        Ok(KeyEncryptionKeyCustodian {
            key_encryption_key,
            key_encryption_key_id,
            cipher: Cipher::Aes256GcmSiv,
        })
    }

    pub fn id(&self) -> Hash {
        self.key_encryption_key_id.clone()
    }
}

impl KeyCustodian for KeyEncryptionKeyCustodian {
    type Secret = MerchantScopedSecret;
    type LockedKey = encryption_key::Model;
    type ActiveLockedKey = encryption_key::ActiveModel;

    fn gen_secret(&self) -> anyhow::Result<[u8; 32]> {
        let mut secret = [0u8; 32];
        thread_rng().fill_bytes(&mut secret);
        Ok(secret)
    }

    fn lock(&self, merchant_secret: Self::Secret) -> anyhow::Result<Self::ActiveLockedKey> {
        let key = Key::from_slice(&self.key_encryption_key);
        let cipher = Aes256GcmSiv::new(key);

        let mut n_raw = vec![0u8; 12];
        thread_rng().fill_bytes(&mut n_raw);
        let nonce = Nonce::from_slice(&n_raw);

        let mut hasher = Sha3_256::new();
        hasher.update(&merchant_secret.secret);
        let id = Hash::try_from(hasher.finalize().to_vec())?;

        let key_encryption_key = cipher
            .encrypt(nonce, merchant_secret.secret.as_ref())
            .map_err(|err| anyhow!("{}", err))?;

        Ok(encryption_key::ActiveModel {
            id: Set(id),
            merchant_id: Set(merchant_secret.merchant_id),
            key_encryption_key_id: Set(self.key_encryption_key_id.clone()),
            cipher: Set(self.cipher.clone()),
            key: Set(key_encryption_key),
            nonce: Set(n_raw),
            created_at: Set(Utc::now()),
        })
    }

    fn unlock(&self, locked_key: Self::LockedKey) -> anyhow::Result<Self::Secret> {
        let key = Key::from_slice(&self.key_encryption_key);
        let cipher = Aes256GcmSiv::new(key);
        let nonce = Nonce::from_slice(&locked_key.nonce);
        let secret = cipher
            .decrypt(nonce, locked_key.key.as_ref())
            .map_err(|err| anyhow!("{}", err))?
            .try_into()
            .map_err(|_| anyhow!("Invalid EncryptionKey"))?;
        Ok(MerchantScopedSecret {
            merchant_id: locked_key.merchant_id,
            secret: secret,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_encryption_key_custodian() {
        let secret = b"an example very very secret key.";
        let kek = KeyEncryptionKeyCustodian::new(secret.to_vec())
            .expect("Invalid MasterKeyEncryptionKeyCustodian");
        assert_eq!(
            kek.id().to_string(),
            "Fgm9dLoNoRgdUwEWB5QLFHdhccYY2Zx5egCrY4gnqJpf".to_string()
        );
        let secret = b"BTH a very very very secret key.";
        let ek = kek.lock(MerchantScopedSecret {
            merchant_id: "merchant_id".to_string(),
            secret: *secret,
        });
        assert!(ek.is_ok());
        let ek = ek.expect("Invalid EncryptionKey");
        let ek = encryption_key::Model {
            id: ek.id.unwrap(),
            merchant_id: ek.merchant_id.unwrap(),
            key_encryption_key_id: ek.key_encryption_key_id.unwrap(),
            cipher: ek.cipher.unwrap(),
            key: ek.key.unwrap(),
            nonce: ek.nonce.unwrap(),
            created_at: ek.created_at.unwrap(),
        };
        let unlocked_secret = kek.unlock(ek);
        assert_eq!(
            unlocked_secret.ok(),
            Some(MerchantScopedSecret {
                merchant_id: "merchant_id".to_string(),
                secret: *secret,
            })
        );
    }
}
