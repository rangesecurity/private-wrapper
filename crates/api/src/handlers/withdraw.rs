use {
    crate::{
        router::AppState,
        types::{ApiError, ApiResponse, Withdraw},
    },
    axum::{extract::State, response::IntoResponse, Json},
    base64::{prelude::BASE64_STANDARD, Engine},
    common::{
        accounts::token_account_already_configured,
        key_generator::{derive_ae_key, derive_elgamal_key, KeypairType},
        proofs::get_zk_proof_context_state_account_creation_instructions,
    },
    http::StatusCode,
    solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction},
    spl_token_2022::{
        extension::{
            confidential_transfer::{
                account_info::{ApplyPendingBalanceAccountInfo, WithdrawAccountInfo},
                instruction::{
                    BatchedRangeProofContext, CiphertextCommitmentEqualityProofContext,
                    ProofContextState,
                },
                ConfidentialTransferAccount,
            },
            BaseStateWithExtensions, StateWithExtensions,
        },
        solana_zk_sdk::zk_elgamal_proof_program::instruction::{
            close_context_state, ContextStateInfo,
        },
    },
    spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation,
    spl_token_confidential_transfer_proof_generation::withdraw::WithdrawProofData,
    std::sync::Arc,
};

/// Handler which is used to withdraw tokens from the confidential balance to the public balance
///
/// # Errors
///
/// * Token acount does not exist and/or not configured for confidential transfers
/// * Insufficient token amount
pub async fn withdraw(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Withdraw>,
) -> impl IntoResponse {
    // derive the ATA for the authority + token_mint
    let user_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payload.authority,
        &payload.token_mint,
        &spl_token_2022::id(),
    );

    // verify elgamal signature
    if !payload.elgamal_signature.verify(
        &payload.authority.to_bytes(),
        &KeypairType::ElGamal.message_to_sign(user_ata),
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "failed to verify elgamal signature".to_string(),
            }),
        )
            .into_response();
    }

    // verify ae signature
    if !payload.ae_signature.verify(
        &payload.authority.to_bytes(),
        &KeypairType::Ae.message_to_sign(user_ata),
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "failed to verify ae signature".to_string(),
            }),
        )
            .into_response();
    }

    // derive the elgamal keypair
    let Ok(elgamal_key) = derive_elgamal_key(payload.elgamal_signature) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to derive elgamal keypair".to_string(),
            }),
        )
            .into_response();
    };

    // derive the ae keypair
    let Ok(ae_key) = derive_ae_key(payload.ae_signature) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to derive ae keypair".to_string(),
            }),
        )
            .into_response();
    };

    // lookup both the token mint, and ata accounts
    let mut accounts = state
        .rpc
        .get_multiple_accounts(&[payload.token_mint, user_ata])
        .await
        .unwrap_or_default();

    // if less than 2 accounts is returned, this means the rpc call failed
    // if the mint does not exist then `accounts[0] == None`
    // if the user_ata does not exist then `accounts[1] == None`
    if accounts.len() < 2 {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to query accounts".to_string(),
            }),
        )
            .into_response();
    };

    // ensure the token mint account exists
    let Some(token_mint) = std::mem::take(&mut accounts[0]) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "token mint does not exist".to_string(),
            }),
        )
            .into_response();
    };

    // ensure token account exists
    let Some(token_account) = std::mem::take(&mut accounts[1]) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "token account does not exist".to_string(),
            }),
        )
            .into_response();
    };

    // optimization note: provide an unpack token account
    // ensure token account is configured for confidential transfers
    if !token_account_already_configured(&token_account) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "token account is not configured for confidential transfers".to_string(),
            }),
        )
            .into_response();
    }

    // get the token mint decimals
    let decimals =
        match StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&token_mint.data) {
            Ok(mint) => mint.base.decimals,
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError {
                        msg: format!("failed to unpack token mint {err:#?}"),
                    }),
                )
                    .into_response()
            }
        };

    // ensure the token mint is valid for confidential transfers
    if !common::accounts::is_valid_mint(token_mint) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "token mint does not support confidential transfers".to_string(),
            }),
        )
            .into_response();
    }

    // unpack token account
    let token_account =
        match StateWithExtensions::<spl_token_2022::state::Account>::unpack(&token_account.data) {
            Ok(token_account) => token_account,
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError {
                        msg: format!("failed to unpack token account {err:#?}"),
                    }),
                )
                    .into_response()
            }
        };

    // retrieve the confidential transfer account extension
    let confidential_transfer_account = match token_account
        .get_extension::<ConfidentialTransferAccount>()
    {
        Ok(confidential_token_account) => confidential_token_account,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to get confidential transfer account extension {err:#?}"),
                }),
            )
                .into_response()
        }
    };

    // Confidential Transfer extension information needed to construct a `Withdraw` instruction.
    let withdraw_account_info = WithdrawAccountInfo::new(confidential_transfer_account);

    println!(
        "available balance {}",
        ae_key
            .decrypt(
                &withdraw_account_info
                    .decryptable_available_balance
                    .try_into()
                    .unwrap()
            )
            .unwrap()
    );

    // Create a withdraw proof data
    let WithdrawProofData {
        equality_proof_data,
        range_proof_data,
    } = match withdraw_account_info.generate_proof_data(payload.amount, &elgamal_key, &ae_key) {
        Ok(proof) => proof,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to generate withdraw proof {err:#?}"),
                }),
            )
                .into_response()
        }
    };

    let range_proof_rent = match state
        .rpc
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            ProofContextState<BatchedRangeProofContext>,
        >())
        .await
    {
        Ok(rent) => rent,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to get range proof rent {err:#?}"),
                }),
            )
                .into_response()
        }
    };

    let equality_proof_rent = match state
        .rpc
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            ProofContextState<CiphertextCommitmentEqualityProofContext>,
        >())
        .await
    {
        Ok(rent) => rent,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to get equality proof rent {err:#?}"),
                }),
            )
                .into_response()
        }
    };

    // Range Proof Instructions------------------------------------------------------------------------------
    let (range_create_ix, range_verify_ix) =
        match get_zk_proof_context_state_account_creation_instructions(
            &payload.authority,
            &payload.range_proof_keypair.pubkey(),
            &payload.authority,
            &range_proof_data,
            range_proof_rent,
        ) {
            Ok(data) => data,
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError {
                        msg: format!("failed to create range proof instructions {err:#?}"),
                    }),
                )
                    .into_response()
            }
        };

    // Equality Proof Instructions---------------------------------------------------------------------------
    let (equality_create_ix, equality_verify_ix) =
        match get_zk_proof_context_state_account_creation_instructions(
            &payload.authority,
            &payload.equality_proof_keypair.pubkey(),
            &payload.authority,
            &equality_proof_data,
            equality_proof_rent,
        ) {
            Ok(data) => data,
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError {
                        msg: format!("failed to create equality proof instructions {err:#?}"),
                    }),
                )
                    .into_response()
            }
        };

    let tx1 = Transaction::new_with_payer(
        &[equality_create_ix, equality_verify_ix, range_create_ix],
        Some(&payload.authority),
    );
    let tx2 = Transaction::new_with_payer(&[range_verify_ix], Some(&payload.authority));

    let tx3 = {
        let Ok(new_decryptable_available_balance) =
            withdraw_account_info.new_decryptable_available_balance(payload.amount, &ae_key)
        else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to encrypt available balance"),
                }),
            )
                .into_response();
        };
        // only way this errors is if incorrect token program is provided
        let instructions = spl_token_2022::extension::confidential_transfer::instruction::withdraw(
            &spl_token_2022::id(),
            &user_ata,
            &payload.token_mint,
            payload.amount,
            decimals,
            &new_decryptable_available_balance.into(),
            &payload.authority,
            &vec![],
            ProofLocation::ContextStateAccount(&payload.equality_proof_keypair.pubkey()),
            ProofLocation::ContextStateAccount(&payload.range_proof_keypair.pubkey()),
        )
        .unwrap();
        Transaction::new_with_payer(&instructions, Some(&payload.authority))
    };

    let tx4 = {
        // Close the equality proof account
        let close_equality_proof_instruction = close_context_state(
            ContextStateInfo {
                context_state_account: &payload.equality_proof_keypair.pubkey(),
                context_state_authority: &payload.authority,
            },
            &payload.authority,
        );

        // Close the range proof account
        let close_range_proof_instruction = close_context_state(
            ContextStateInfo {
                context_state_account: &payload.range_proof_keypair.pubkey(),
                context_state_authority: &payload.authority,
            },
            &payload.authority,
        );
        Transaction::new_with_payer(
            &[
                close_equality_proof_instruction,
                close_range_proof_instruction,
            ],
            Some(&payload.authority),
        )
    };
    let txs = [tx1, tx2, tx3, tx4]
        .into_iter()
        .filter_map(|tx| Some(BASE64_STANDARD.encode(bincode::serialize(&tx).ok()?)))
        .collect::<Vec<_>>();

    if txs.len() != 4 {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to encode transactions".to_string(),
            }),
        )
            .into_response();
    }
    (StatusCode::OK, Json(ApiResponse { transactions: txs })).into_response()
}
