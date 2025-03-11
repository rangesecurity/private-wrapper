//! Utilities for vworking with EGamal and Ae keys used with confidential transfers

use {
    anyhow::{anyhow, Result},
    solana_sdk::{pubkey::Pubkey, signature::Signature},
    solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
};

/// Defines the two types of keypairs that are required by confidential transactions
#[derive(Clone, Copy)]
pub enum KeypairType {
    ElGamal,
    Ae,
}

impl KeypairType {
    /// Returns the message to sign to generate the corresponding keypair
    pub fn message_to_sign(self, user_ata: Pubkey) -> Vec<u8> {
        match self {
            Self::ElGamal => [b"ElGamalSecretKey", &user_ata.to_bytes()[..]].concat(),
            Self::Ae => [b"AEKey", &user_ata.to_bytes()[..]].concat(),
        }
    }
}

/// Derives an ElGamal key from a signature
pub fn derive_elgamal_key(signature: Signature) -> Result<ElGamalKeypair> {
    ElGamalKeypair::new_from_signature(&signature)
        .map_err(|e| anyhow!("failed to derive elgamal keypair {e:#?}"))
}

/// Derives an Ae key from a signature
pub fn derive_ae_key(signature: Signature) -> Result<AeKey> {
    AeKey::new_from_signature(&signature).map_err(|e| anyhow!("failed to derive ae keypair {e:#?}"))
}

#[cfg(test)]
mod test {
    use crate::test_helpers::test_key;
    use solana_sdk::{signature::Keypair, signer::Signer};

    use super::*;

    #[test]
    fn test_derive_elgamal_keypair() {
        let user_key = test_key();

        let user_ata = Pubkey::new_unique();

        let kt = KeypairType::ElGamal;

        let msg = kt.message_to_sign(user_ata);
        assert_eq!(msg.len(), 48);

        let signature = user_key.sign_message(&msg);

        let elgamal_key = derive_elgamal_key(signature).unwrap();

        let elgamal_pubkey = elgamal_key.pubkey().to_string();

        // ernsure the pubkey of the keypair we generated is expected
        assert_eq!(
            elgamal_pubkey,
            "yK6ZeLGATEB+S/gR2xcNWotmym2AXeaM+1U0exawHB0="
        );
    }

    #[test]
    fn test_derive_ae_key() {
        let user_key = test_key();

        let user_ata = Pubkey::new_unique();

        let kt = KeypairType::Ae;

        let msg = kt.message_to_sign(user_ata);

        let signature = user_key.sign_message(&msg);

        let ae_key = derive_ae_key(signature).unwrap();

        let key: [u8; 16] = From::from(ae_key);

        assert_eq!(
            key,
            [21, 34, 125, 137, 145, 57, 110, 58, 128, 240, 23, 134, 231, 8, 47, 23]
        );
    }
}
