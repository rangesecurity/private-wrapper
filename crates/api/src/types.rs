use {
    crate::serde_utils,
    anyhow::Context,
    base64::{prelude::BASE64_STANDARD, Engine},
    serde::{Deserialize, Serialize},
    solana_sdk::{pubkey::Pubkey, signature::Signature, transaction::Transaction},
};

/// JSON request used to initialize a confidential token account
#[derive(Serialize, Deserialize)]
pub struct Initialize {
    /// The public key of the wallet which generated the signatures
    #[serde(with = "serde_utils::pubkey_string")]
    pub authority: Pubkey,
    /// The confidential token mint for which we are initializing an account for
    #[serde(with = "serde_utils::pubkey_string")]
    pub token_mint: Pubkey,
    /// The signed message of [b"ElGamalSecretKey", user_ata]
    ///
    /// This is used to derive the ElGamal keypair
    #[serde(with = "serde_utils::signature_string")]
    pub elgamal_signature: Signature,
    /// The signed message of [b"AEKey", user_ata]
    ///
    /// This is used to derive the AE key
    #[serde(with = "serde_utils::signature_string")]
    pub ae_signature: Signature,
}

/// JSON request used to deposit from non-confidential balance to pending balance
///
#[derive(Serialize, Deserialize)]
pub struct DepositOrWithdraw {
    /// The public key of the wallet which is depositing tokens
    #[serde(with = "serde_utils::pubkey_string")]
    pub authority: Pubkey,
    /// The confidential token mint
    #[serde(with = "serde_utils::pubkey_string")]
    pub token_mint: Pubkey,
    /// The signed message of [b"ElGamalSecretKey", user_ata]
    ///
    /// This is used to derive the ElGamal keypair
    #[serde(with = "serde_utils::signature_string")]
    pub elgamal_signature: Signature,
    /// The signed message of [b"AEKey", user_ata]
    ///
    /// This is used to derive the AE key
    #[serde(with = "serde_utils::signature_string")]
    pub ae_signature: Signature,
    /// The amount of tokens to deposit or withdraw in lamports
    pub amount: u64,
}

/// JSON response indicating an error message
#[derive(Serialize, Deserialize)]
pub struct ApiError {
    pub msg: String,
}

/// JSON response containing one or more transactions
#[derive(Serialize, Deserialize)]
pub struct ApiResponse {
    /// Transactions returned by the confidential blink api
    ///
    /// If multiple transactions are returned, they must be executed in sequence
    pub transactions: Vec<String>,
}

impl ApiResponse {
    /// Returns a vec of decoded transactions, consuming the response
    pub fn decode_transactions(self) -> anyhow::Result<Vec<Transaction>> {
        let mut transactions = Vec::with_capacity(self.transactions.len());
        for tx in self.transactions {
            transactions.push(
                bincode::deserialize(
                    &BASE64_STANDARD
                        .decode(tx)
                        .with_context(|| "failed to decode transaction")?,
                )
                .with_context(|| "failed to deserialize transaction")?,
            );
        }
        Ok(transactions)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use common::{key_generator::KeypairType, test_helpers::test_key};
    use solana_sdk::signer::Signer;

    #[test]
    fn test_initialize_serialization() {
        let key = test_key();
        let mint = Pubkey::new_unique();
        let expected_elgamal_signature =
            key.sign_message(&KeypairType::ElGamal.message_to_sign(key.pubkey()));
        let expected_ae_signature =
            key.sign_message(&KeypairType::Ae.message_to_sign(key.pubkey()));

        let init_msg: Initialize = serde_json::from_value(serde_json::json!({
            "authority": key.pubkey().to_string(),
            "token_mint": mint.to_string(),
            "elgamal_signature": expected_elgamal_signature.to_string(),
            "ae_signature": expected_ae_signature.to_string()
        }))
        .unwrap();

        assert_eq!(init_msg.authority, key.pubkey(),);
        assert_eq!(init_msg.elgamal_signature, expected_elgamal_signature);
        assert_eq!(init_msg.ae_signature, expected_ae_signature);
        assert_eq!(init_msg.token_mint, mint);
    }
}
