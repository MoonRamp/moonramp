use moonramp_core::anyhow;

mod encryption_key;
mod key_encryption_key;
mod master_key_encryption_key;

pub use encryption_key::*;
pub use key_encryption_key::*;
pub use master_key_encryption_key::*;

pub trait KeyCustodian {
    type Secret;
    type LockedKey;
    type ActiveLockedKey;

    fn gen_secret(&self) -> anyhow::Result<[u8; 32]>;
    fn lock(&self, secret: Self::Secret) -> anyhow::Result<Self::ActiveLockedKey>;
    fn unlock(&self, locked_key: Self::LockedKey) -> anyhow::Result<Self::Secret>;
}
