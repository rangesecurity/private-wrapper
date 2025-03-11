//! Utilities for working with solana accounts in the context of confidential transfers

use {
    anyhow::{Context, Result},
    solana_sdk::account::Account,
    spl_token_2022::{
        extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions},
        state::Mint,
    },
};

/// Checks to see if the specified account is a valid token mint for confidential transfers
///
/// Validates that:
/// * The account is an spl_token_2022 mint account
/// * Supports the ConfidentialTransferMint extension
pub fn is_valid_mint(mint: Account) -> bool {
    // unpack the token account
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


#[cfg(test)]
mod test {
    use solana_sdk::{program_pack::Pack, pubkey::Pubkey};
    use spl_pod::{optional_keys::OptionalNonZeroPubkey, primitives::PodBool};
    use spl_token_2022::extension::{confidential_transfer::ConfidentialTransferMint, BaseStateWithExtensionsMut, StateWithExtensionsMut};

    use super::*;

    #[test]
    fn test_is_valid_mint_false() {
        let mut account_data = vec![0; Mint::LEN];
        let mint = Mint {
            supply: 1_000,
            ..Default::default()
        };
        Mint::pack(mint, &mut account_data).unwrap();

        assert!(
            !is_valid_mint(Account {
                data: account_data,
                ..Default::default()
            })
        );
    }

    #[test]
    fn test_is_valid_mint_not_token_account() {
        assert!(
            is_valid_mint(
                Account {
                    data: vec![1, 2, 3, 4],
                    ..Default::default()
                }
            )
        )
    }

    #[test]
    fn test_is_valid_mint() {
        let account_size = ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::ConfidentialTransferMint]).unwrap();
        let mut account_data = vec![0; account_size];

        let mut state = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut account_data).unwrap();

        let extension = state.init_extension::<ConfidentialTransferMint>(false).unwrap();

        extension.auto_approve_new_accounts = PodBool::from_bool(true);
        extension.authority = OptionalNonZeroPubkey::try_from(Some(Pubkey::new_unique())).unwrap();
        
        state.base = Mint {
            supply: 1_000,
            is_initialized: true,
            ..Default::default()
        };

        state.pack_base();
        state.init_account_type().unwrap();

        assert!(
            is_valid_mint(
                Account {
                    data: account_data,
                    ..Default::default()
                }
            )
        )
    }
}