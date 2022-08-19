use std::str::FromStr;

use actix_web::web;

pub async fn send_transfer(
    rpc_client: web::Data<near_jsonrpc_client::JsonRpcClient>,
    web::Json(request): web::Json<super::schemas::SendTransferRequest>,
) -> crate::Result<web::Json<super::schemas::SendTransferResponse>> {
    let derived_private_key = slip10::derive_key_from_path(
        &request.sender_seed_phrase.to_seed(""),
        slip10::Curve::Ed25519,
        &slip10::BIP32Path::from_str("m/44'/397'/0'").unwrap(),
    )
    .map_err(|err| {
        crate::errors::ErrorKind::InvalidInput(format!(
            "Failed to derive a key from the master key: {}",
            err
        ))
    })?;

    let secret_keypair = {
        let secret = ed25519_dalek::SecretKey::from_bytes(&derived_private_key.key).unwrap();
        let public = ed25519_dalek::PublicKey::from(&secret);
        ed25519_dalek::Keypair { secret, public }
    };

    let sender_account_id = if let Some(sender_account_id) = request.sender_account_id {
        let request = near_jsonrpc_client::methods::query::RpcQueryRequest {
            block_reference: near_primitives::types::Finality::Final.into(),
            request: near_primitives::views::QueryRequest::ViewAccessKey {
                account_id: sender_account_id.clone(),
                public_key: near_crypto::ED25519PublicKey::from(
                    secret_keypair.public.clone().to_bytes(),
                )
                .into(),
            },
        };

        let response = rpc_client.call(request).await?;

        if let near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKey(_) =
            response.kind
        {
            sender_account_id
        } else {
            return Err(crate::errors::ErrorKind::InvalidInput(format!(
                "The sender_seed_phrase does not match any access key on the {} account",
                sender_account_id
            ))
            .into());
        }
    } else {
        near_primitives::types::AccountId::try_from(hex::encode(&secret_keypair.public))?
    };

    // TODO: check if balance available

    let signer_secret_key = std::env::var("CASTODIAL_SIGNER_SECRET_KEY")
        .expect("CASTODIAL_SIGNER_SECRET_KEY environment variable is not provided")
        .parse()
        .unwrap();
    let signer = near_crypto::InMemorySigner::from_secret_key(
        std::env::var("CASTODIAL_SIGNER_ACCOUNT_ID")
            .expect("CASTODIAL_SIGNER_ACCOUNT_ID environment variable is not provided")
            .parse()
            .unwrap(),
        signer_secret_key,
    );

    let access_key_query_response = rpc_client
        .call(near_jsonrpc_client::methods::query::RpcQueryRequest {
            block_reference: near_primitives::types::Finality::Final.into(),
            request: near_primitives::views::QueryRequest::ViewAccessKey {
                account_id: signer.account_id.clone(),
                public_key: signer.public_key.clone(),
            },
        })
        .await?;

    let current_nonce = match access_key_query_response.kind {
        near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKey(access_key) => {
            access_key.nonce
        }
        _ => Err("failed to extract current nonce")?,
    };

    let transaction = near_primitives::transaction::Transaction {
        signer_id: signer.account_id.clone(),
        public_key: signer.public_key.clone(),
        nonce: current_nonce + 1,
        receiver_id: signer.account_id.clone(),
        block_hash: access_key_query_response.block_hash,
        actions: vec![near_primitives::transaction::Action::FunctionCall(
            near_primitives::transaction::FunctionCallAction {
                method_name: "custodial_ft_transfer".to_string(),
                args: serde_json::json!({
                    "sender_id": sender_account_id.to_string(),
                    "receiver_id": request.receiver_account_id.to_string(),
                    "amount": request.transfer_amount,
                })
                .to_string()
                .into_bytes(),
                gas: 10_000_000_000_000, // 10 TeraGas
                deposit: 0,
            },
        )],
    };

    let request = near_jsonrpc_client::methods::broadcast_tx_commit::RpcBroadcastTxCommitRequest {
        signed_transaction: transaction.sign(&signer),
    };

    let transaction_hash = {
        let mut attemps = 10;
        loop {
            let response = rpc_client.call(&request).await?;
            if let near_primitives::views::FinalExecutionStatus::SuccessValue(_) = response.status {
                break Some(response.transaction.hash);
            }
            if attemps == 0 {
                break None;
            }
            attemps -= 1;
        }
    };

    Ok(web::Json(super::schemas::SendTransferResponse {
        status: "ok".to_string(),
        error_message: None,
        transaction_hash,
    }))
}
