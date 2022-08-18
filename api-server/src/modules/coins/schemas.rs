#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SendTransferRequest {
    // TODO
    //signed_transaction,
    // XXX
    pub transfer_amount: crate::integers::U128,
    pub sender_seed_phrase: bip39::Mnemonic,
    pub receiver_account_id: near_primitives::types::AccountId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SendTransferResponse {
    pub status: String,
    pub error_message: Option<String>,
    pub transaction_hash: Option<near_primitives::hash::CryptoHash>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Reward {
    target_account_id: near_primitives::types::AccountId,
    tokens_amount: crate::integers::U128,
    memo: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecretRewardRequest {
    rewards: Vec<Reward>,
    memo: Option<String>,
}
