use {
    serde::{Deserialize, Serialize}, solana_sdk::{pubkey::Pubkey, signature::Signature}
};

/// JSON request used to initialize a confidential token account
#[derive(Serialize, Deserialize)]
pub struct Initialize {
    /// The public key of the wallet which generated the signatures
    #[serde(with = "pubkey_string")]
    pub authority: Pubkey,
    /// The signed message of [b"ElGamalSecretKey", user_ata]
    /// 
    /// This is used to derive the ElGamal keypair
    #[serde(with = "signature_string")]
    pub elgamal_signature: Signature,
    /// The signed message of [b"AEKey", user_ata]
    /// 
    /// This is used to derive the AE key
    #[serde(with = "signature_string")]
    pub ae_signature: Signature,
}

/// JSON response indicating an error message
#[derive(Serialize, Deserialize)]
pub struct ApiError {
    pub msg: String,
}

mod pubkey_string {
    use {
        serde::{Deserialize, Deserializer, Serializer}, solana_sdk::pubkey::Pubkey, std::str::FromStr
    };

    pub fn serialize<S>(pubkey: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&pubkey.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Pubkey::from_str(&s).map_err(serde::de::Error::custom)
    }
}

mod signature_string {
    use {
        serde::{Deserialize, Deserializer, Serializer}, solana_sdk::signature::Signature, std::str::FromStr
    };

    pub fn serialize<S>(signature: &Signature, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&signature.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Signature, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Signature::from_str(&s).map_err(serde::de::Error::custom)
    }
}


#[cfg(test)]
mod test {
    use common::{key_generator::KeypairType, test_helpers::test_key};
    use solana_sdk::signer::Signer;
    use super::*;

    #[test]
    fn test_initialize_serialization() {
        let key = test_key();
        let expected_elgamal_signature = key.sign_message(&KeypairType::ElGamal.message_to_sign(key.pubkey()));
        let expected_ae_signature = key.sign_message(&KeypairType::Ae.message_to_sign(key.pubkey()));

        let init_msg: Initialize = serde_json::from_value(serde_json::json!({
            "authority": key.pubkey().to_string(),
            "elgamal_signature": expected_elgamal_signature.to_string(),
            "ae_signature": expected_ae_signature.to_string()
        })).unwrap();

        assert_eq!(
            init_msg.authority, key.pubkey(),
        );
        assert_eq!(
            init_msg.elgamal_signature, expected_elgamal_signature
        );
        assert_eq!(
            init_msg.ae_signature, expected_ae_signature
        );
    }
}