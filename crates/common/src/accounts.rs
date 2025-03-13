//! Utilities for working with solana accounts in the context of confidential transfers

use {
    solana_sdk::account::Account,
    spl_token_2022::{
        extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions},
        state::{Account as TokenAccount, Mint},
    },
};

/// Checks to see if the specified account is a valid token mint for confidential transfers
///
/// Validates that:
/// * The account is an spl_token_2022 mint account
/// * Supports the ConfidentialTransferMint extension
pub fn is_valid_mint(mint: &Account) -> bool {
    // unpack the mint account
    let Ok(state) = StateWithExtensions::<Mint>::unpack(&mint.data) else {
        return false;
    };

    // lookup extensions
    let Ok(extensions) = state.get_extension_types() else {
        return false;
    };

    // check to see if the ConfidentialTransferMint extension is supported
    extensions
        .into_iter()
        .find(|ext| ext.eq(&ExtensionType::ConfidentialTransferMint))
        .is_some()
}

/// Checks to see if the token account is already configured for confidential transfers
///
/// Validates that:
/// * The account is an spl_token_2022 token account
/// * Supports the ConfidentialTransferAccount extension
pub fn token_account_already_configured(account: &Account) -> bool {
    let Ok(state) = StateWithExtensions::<TokenAccount>::unpack(&account.data) else {
        return false;
    };

    let Ok(extensions) = state.get_extension_types() else {
        return false;
    };

    extensions
        .into_iter()
        .find(|ext| ext.eq(&ExtensionType::ConfidentialTransferAccount))
        .is_some()
}

#[cfg(test)]
mod test {
    use {
        super::*,
        bytemuck::Zeroable,
        solana_sdk::{program_pack::Pack, pubkey::Pubkey},
        solana_zk_sdk::encryption::{auth_encryption::AeKey, elgamal::ElGamalKeypair},
        spl_pod::{optional_keys::OptionalNonZeroPubkey, primitives::PodBool},
        spl_token_2022::{
            extension::{
                confidential_transfer::{
                    ConfidentialTransferAccount, ConfidentialTransferMint, EncryptedBalance,
                },
                BaseStateWithExtensionsMut, StateWithExtensionsMut,
            },
            state::AccountState,
        },
    };

    #[test]
    fn test_is_valid_mint_false() {
        let mut account_data = vec![0; Mint::LEN];
        let mint = Mint {
            supply: 1_000,
            ..Default::default()
        };
        Mint::pack(mint, &mut account_data).unwrap();

        assert!(!is_valid_mint(&Account {
            data: account_data,
            ..Default::default()
        }));
    }

    #[test]
    fn test_is_valid_mint_not_a_mint() {
        assert!(!is_valid_mint(&Account {
            data: vec![1, 2, 3, 4],
            ..Default::default()
        }))
    }

    #[test]
    fn test_is_valid_mint() {
        let account_size = ExtensionType::try_calculate_account_len::<Mint>(&[
            ExtensionType::ConfidentialTransferMint,
        ])
        .unwrap();
        let mut account_data = vec![0; account_size];

        let mut state =
            StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut account_data).unwrap();

        let extension = state
            .init_extension::<ConfidentialTransferMint>(false)
            .unwrap();

        extension.auto_approve_new_accounts = PodBool::from_bool(true);
        extension.authority = OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique())).unwrap();

        state.base = Mint {
            supply: 1_000,
            is_initialized: true,
            ..Default::default()
        };

        state.pack_base();
        state.init_account_type().unwrap();

        assert!(is_valid_mint(&Account {
            data: account_data,
            ..Default::default()
        }))
    }

    #[test]
    fn test_token_account_already_configured_not_a_token() {
        assert!(!token_account_already_configured(&Account {
            data: vec![1, 2, 3],
            ..Default::default()
        }));
    }

    #[test]
    fn test_token_account_already_configured_unconfigured() {
        let mut account_data = vec![0; TokenAccount::LEN];
        TokenAccount::pack(
            TokenAccount {
                mint: Pubkey::new_unique(),
                owner: Pubkey::new_unique(),
                amount: 123,
                ..Default::default()
            },
            &mut account_data,
        )
        .unwrap();
        assert!(!token_account_already_configured(&Account {
            data: account_data,
            ..Default::default()
        }));
    }

    #[test]
    fn test_token_account_already_configured() {
        let account_size = ExtensionType::try_calculate_account_len::<TokenAccount>(&[
            ExtensionType::ConfidentialTransferAccount,
        ])
        .unwrap();

        let mut account_data = vec![0; account_size];

        let mut state =
            StateWithExtensionsMut::<TokenAccount>::unpack_uninitialized(&mut account_data)
                .unwrap();

        let elgamal_keypair = ElGamalKeypair::new_rand();
        let ae_keypair = AeKey::new_rand();

        let extension = state
            .init_extension::<ConfidentialTransferAccount>(false)
            .unwrap();
        extension.approved = PodBool::from_bool(true);
        extension.elgamal_pubkey = elgamal_keypair.pubkey_owned().into();
        extension.maximum_pending_balance_credit_counter = 65536.into();

        extension.pending_balance_lo = EncryptedBalance::zeroed();
        extension.pending_balance_hi = EncryptedBalance::zeroed();
        extension.available_balance = EncryptedBalance::zeroed();

        extension.decryptable_available_balance = ae_keypair.encrypt(0).into();
        extension.allow_confidential_credits = PodBool::from_bool(true);
        extension.expected_pending_balance_credit_counter = 0.into();
        extension.actual_pending_balance_credit_counter = 0.into();
        extension.allow_non_confidential_credits = PodBool::from_bool(true);

        state.base = TokenAccount {
            mint: Pubkey::new_unique(),
            state: AccountState::Initialized,
            ..Default::default()
        };
        state.pack_base();
        state.init_account_type().unwrap();

        assert!(token_account_already_configured(&Account {
            data: account_data,
            ..Default::default()
        }))
    }
}
