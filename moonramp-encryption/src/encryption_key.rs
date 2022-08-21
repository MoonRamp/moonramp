use anyhow::anyhow;
use rand::{thread_rng, RngCore};
use sha3::{Digest, Sha3_256};

use moonramp_core::{aes_gcm_siv, anyhow, chacha20poly1305, rand, sha3, Hash};
use moonramp_entity::cipher::Cipher;

pub struct EncryptionKeyCustodian {
    encryption_key: Vec<u8>,
    encryption_key_hash: Hash,
    cipher: Cipher,
}

impl EncryptionKeyCustodian {
    pub fn new(encryption_key: Vec<u8>, cipher: Cipher) -> anyhow::Result<Self> {
        let mut hasher = Sha3_256::new();
        hasher.update(&encryption_key);
        let encryption_key_hash = Hash::try_from(hasher.finalize().to_vec())?;

        Ok(EncryptionKeyCustodian {
            encryption_key,
            encryption_key_hash,
            cipher,
        })
    }

    pub fn hash(&self) -> Hash {
        self.encryption_key_hash.clone()
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
        match self.cipher {
            Cipher::Aes256GcmSiv => {
                use aes_gcm_siv::{
                    aead::{Aead, NewAead},
                    Aes256GcmSiv, Key, Nonce,
                };

                let key = Key::from_slice(&self.encryption_key);
                let cipher = Aes256GcmSiv::new(key);

                let mut n_raw = vec![0u8; 12];
                thread_rng().fill_bytes(&mut n_raw);
                let nonce = Nonce::from_slice(&n_raw);

                let ciphertext = cipher
                    .encrypt(nonce, plaintext.as_ref())
                    .map_err(|err| anyhow!("{}", err))?;

                Ok((n_raw, ciphertext))
            }
            Cipher::ChaCha20Poly1305 => {
                use chacha20poly1305::{
                    aead::{Aead, NewAead},
                    ChaCha20Poly1305, Key, Nonce,
                };

                let key = Key::from_slice(&self.encryption_key);
                let cipher = ChaCha20Poly1305::new(key);

                let mut n_raw = vec![0u8; 12];
                thread_rng().fill_bytes(&mut n_raw);
                let nonce = Nonce::from_slice(&n_raw);

                let ciphertext = cipher
                    .encrypt(nonce, plaintext.as_ref())
                    .map_err(|err| anyhow!("{}", err))?;

                Ok((n_raw, ciphertext))
            }
            #[cfg(feature = "testing")]
            Cipher::Noop => {
                let mut n_raw = vec![0u8; 12];
                thread_rng().fill_bytes(&mut n_raw);
                Ok((n_raw, plaintext.to_vec()))
            }
        }
    }

    pub fn decrypt(&self, nonce: &[u8], ciphertext: &[u8]) -> anyhow::Result<Vec<u8>> {
        match self.cipher {
            Cipher::Aes256GcmSiv => {
                use aes_gcm_siv::{
                    aead::{Aead, NewAead},
                    Aes256GcmSiv, Key, Nonce,
                };

                let key = Key::from_slice(&self.encryption_key);
                let cipher = Aes256GcmSiv::new(key);
                let nonce = Nonce::from_slice(nonce);
                Ok(cipher
                    .decrypt(nonce, ciphertext.as_ref())
                    .map_err(|err| anyhow!("{}", err))?
                    .try_into()
                    .map_err(|_| anyhow!("Invalid EncryptionKey"))?)
            }
            Cipher::ChaCha20Poly1305 => {
                use chacha20poly1305::{
                    aead::{Aead, NewAead},
                    ChaCha20Poly1305, Key, Nonce,
                };

                let key = Key::from_slice(&self.encryption_key);
                let cipher = ChaCha20Poly1305::new(key);
                let nonce = Nonce::from_slice(nonce);
                Ok(cipher
                    .decrypt(nonce, ciphertext.as_ref())
                    .map_err(|err| anyhow!("{}", err))?
                    .try_into()
                    .map_err(|_| anyhow!("Invalid EncryptionKey"))?)
            }
            #[cfg(feature = "testing")]
            Cipher::Noop => Ok(ciphertext.to_vec()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_key_custodian() {
        let secret = b"an example very very secret key.";
        let ek = EncryptionKeyCustodian::new(secret.to_vec(), Cipher::ChaCha20Poly1305)
            .expect("Invalid MasterKeyEncryptionKeyCustodian");
        assert_eq!(
            ek.hash().to_string(),
            "Fgm9dLoNoRgdUwEWB5QLFHdhccYY2Zx5egCrY4gnqJpf".to_string()
        );
        let res = ek.encrypt(&[0x01, 0x02]);
        assert!(res.is_ok());
    }
}
