use {
    crate::{
        router::AppState,
        types::{ApiError, ApiResponse, Transfer, Withdraw},
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
                account_info::{
                    ApplyPendingBalanceAccountInfo, TransferAccountInfo, WithdrawAccountInfo,
                },
                instruction::{
                    BatchedGroupedCiphertext3HandlesValidityProofContext, BatchedRangeProofContext,
                    CiphertextCommitmentEqualityProofContext, ProofContextState,
                },
                ConfidentialTransferAccount, ConfidentialTransferMint,
            },
            BaseStateWithExtensions, StateWithExtensions,
        },
        solana_zk_sdk::{
            encryption::{elgamal::ElGamalPubkey, pod::elgamal::PodElGamalPubkey},
            zk_elgamal_proof_program::instruction::{close_context_state, ContextStateInfo},
        },
    },
    spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation,
    spl_token_confidential_transfer_proof_generation::{
        transfer::TransferProofData, withdraw::WithdrawProofData,
    },
    std::sync::Arc,
};

pub async fn transfer(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Transfer>,
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
        .get_multiple_accounts(&[
            payload.token_mint,
            user_ata,
            payload.receiving_token_account,
        ])
        .await
        .unwrap_or_default();

    // if less than 3 accounts is returned, this means the rpc call failed
    // if the mint does not exist then `accounts[0] == None`
    // if the user_ata does not exist then `accounts[1] == None`
    // if the receiving ata does not exist then  `accounts[2] == None`
    if accounts.len() < 3 {
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
    let Some(sender_token_account) = std::mem::take(&mut accounts[1]) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "authority token account does not exist".to_string(),
            }),
        )
            .into_response();
    };

    let Some(receiving_token_account) = std::mem::take(&mut accounts[2]) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "receiving token account does not exist".to_string(),
            }),
        )
            .into_response();
    };

    // optimization note: provide an unpack token account
    // ensure token account is configured for confidential transfers
    if !token_account_already_configured(&sender_token_account) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "authority token account is not configured for confidential transfers"
                    .to_string(),
            }),
        )
            .into_response();
    }

    if !token_account_already_configured(&receiving_token_account) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "authority token account is not configured for confidential transfers"
                    .to_string(),
            }),
        )
            .into_response();
    }

    // ensure the token mint is valid for confidential transfers
    if !common::accounts::is_valid_mint(&token_mint) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                msg: "token mint does not support confidential transfers".to_string(),
            }),
        )
            .into_response();
    }

    // get the token mint decimals
    let mint_account =
        match StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&token_mint.data) {
            Ok(mint) => mint,
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

    let mint_extension = match mint_account.get_extension::<ConfidentialTransferMint>() {
        Ok(mint) => mint,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to get confidential transfer mint extension {err:#?}"),
                }),
            )
                .into_response()
        }
    };

    let sender_token_account = match StateWithExtensions::<spl_token_2022::state::Account>::unpack(
        &sender_token_account.data,
    ) {
        Ok(token_account) => token_account,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: format!("failed to unpack authority token account {err:#?}"),
                }),
            )
                .into_response()
        }
    };
    let receiving_token_account =
        match StateWithExtensions::<spl_token_2022::state::Account>::unpack(
            &receiving_token_account.data,
        ) {
            Ok(token_account) => token_account,
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError {
                        msg: format!("failed to unpack receiving token account {err:#?}"),
                    }),
                )
                    .into_response()
            }
        };
    let sender_confidential_transfer_account = match sender_token_account
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

    let receiving_confidential_transfer_account = match receiving_token_account
        .get_extension::<ConfidentialTransferAccount>(
    ) {
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
    let auditor_pubkey = if let Some(auditor_pubkey) =
        Option::<PodElGamalPubkey>::from(mint_extension.auditor_elgamal_pubkey)
    {
        let Ok(auditor_pubkey) = TryInto::<ElGamalPubkey>::try_into(auditor_pubkey) else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: "failed to parse auditor pubkey".to_string(),
                }),
            )
                .into_response();
        };
        Some(auditor_pubkey)
    } else {
        None
    };

    let Ok(destination_pubkey) =
        TryInto::<ElGamalPubkey>::try_into(receiving_confidential_transfer_account.elgamal_pubkey)
    else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to parse destiantion pubkey".to_string(),
            }),
        )
            .into_response();
    };

    let sender_transfer_account = TransferAccountInfo::new(sender_confidential_transfer_account);
    // bit awkward since split proof generation takes a type Option<&>
    let transfer_proof_data = if let Some(auditor_pubkey) =
        Option::<PodElGamalPubkey>::from(mint_extension.auditor_elgamal_pubkey)
    {
        let Ok(auditor_pubkey) = TryInto::<ElGamalPubkey>::try_into(auditor_pubkey) else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: "failed to parse auditor pubkey".to_string(),
                }),
            )
                .into_response();
        };
        sender_transfer_account.generate_split_transfer_proof_data(
            payload.amount,
            &elgamal_key,
            &ae_key,
            &destination_pubkey,
            Some(&auditor_pubkey),
        )
    } else {
        sender_transfer_account.generate_split_transfer_proof_data(
            payload.amount,
            &elgamal_key,
            &ae_key,
            &destination_pubkey,
            None,
        )
    };

    let Ok(TransferProofData {
        equality_proof_data,
        ciphertext_validity_proof_data_with_ciphertext,
        range_proof_data,
    }) = transfer_proof_data
    else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                msg: "failed to generate split transfer proof".to_string(),
            }),
        )
            .into_response();
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

    let ciphertext_proof_rent = match state
        .rpc
        .get_minimum_balance_for_rent_exemption(std::mem::size_of::<
            ProofContextState<BatchedGroupedCiphertext3HandlesValidityProofContext>,
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

    // Must first create 3 accounts to store proofs before transferring tokens
    // This must be done in a separate transactions because the proofs are too large for single transaction:
    // Equality Proof - prove that two ciphertexts encrypt the same value
    // Ciphertext Validity Proof - prove that ciphertexts are properly generated
    // Range Proof - prove that ciphertexts encrypt a value in a specified range (0, u64::MAX)

    // Create 3 proofs ------------------------------------------------------

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

    // Ciphertext Validity Proof Instructions ----------------------------------------------------------------
    let (cv_create_ix, cv_verify_ix) =
        match get_zk_proof_context_state_account_creation_instructions(
            &payload.authority,
            &payload.ciphertext_validity_proof_keypair.pubkey(),
            &payload.authority,
            &ciphertext_validity_proof_data_with_ciphertext.proof_data,
            ciphertext_proof_rent,
        ) {
            Ok(data) => data,
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError {
                        msg: format!("failed to create ciphertext proof instructions {err:#?}"),
                    }),
                )
                    .into_response()
            }
        };

    // Transaction 1: Allocate all proof accounts at once.
    let tx1 = {
        // Create instructions vector
        let mut instructions = Vec::new();

        // Add the original instructions
        instructions.push(range_create_ix.clone());
        instructions.push(equality_create_ix.clone());
        instructions.push(cv_create_ix.clone());

        Transaction::new_with_payer(
            &[range_create_ix, equality_create_ix, cv_create_ix],
            Some(&payload.authority),
        )
    };

    // Transaction 2: Encode Range Proof on its own (because it's the largest).
    let tx2 = Transaction::new_with_payer(&[range_verify_ix], Some(&payload.authority));
    let tx3 = Transaction::new_with_payer(
        &[equality_verify_ix, cv_verify_ix],
        Some(&payload.authority),
    );

    // Transaction 4: Execute transfer (below)
    // Transfer with Split Proofs -------------------------------------------
    let tx4 = {
        let Ok(new_decryptable_available_balance) =
            sender_transfer_account.new_decryptable_available_balance(payload.amount, &ae_key)
        else {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    msg: "failed to encrypt available balance".to_string(),
                }),
            )
                .into_response();
        };

        // can only fail if incorrect token program is provided
        let instructions = spl_token_2022::extension::confidential_transfer::instruction::transfer(
            &spl_token_2022::id(),
            &user_ata,
            &payload.token_mint,
            &payload.receiving_token_account,
            &new_decryptable_available_balance.into(),
            &ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo,
            &ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi,
            &payload.authority,
            &vec![],
            ProofLocation::ContextStateAccount(&payload.equality_proof_keypair.pubkey()),
            ProofLocation::ContextStateAccount(&payload.ciphertext_validity_proof_keypair.pubkey()),
            ProofLocation::ContextStateAccount(&payload.range_proof_keypair.pubkey()),
        )
        .unwrap();

        Transaction::new_with_payer(&instructions, Some(&payload.authority))
    };

    // Transaction 5: (below)
    // Close Proof Accounts --------------------------------------------------
    let tx5 = {
        // Close the equality proof account
        let close_equality_proof_instruction = close_context_state(
            ContextStateInfo {
                context_state_account: &payload.equality_proof_keypair.pubkey(),
                context_state_authority: &payload.authority,
            },
            &payload.authority,
        );

        // Close the ciphertext validity proof account
        let close_ciphertext_validity_proof_instruction = close_context_state(
            ContextStateInfo {
                context_state_account: &payload.ciphertext_validity_proof_keypair.pubkey(),
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
                close_ciphertext_validity_proof_instruction,
                close_range_proof_instruction,
            ],
            Some(&payload.authority),
        )
    };
    let txs = [tx1, tx2, tx3, tx4, tx5]
        .into_iter()
        .filter_map(|tx| Some(BASE64_STANDARD.encode(bincode::serialize(&tx).ok()?)))
        .collect::<Vec<_>>();

    if txs.len() != 5 {
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
