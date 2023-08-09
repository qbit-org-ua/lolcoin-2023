/*!
Non-Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_contract_standards::non_fungible_token::approval::NonFungibleTokenApproval;
use near_contract_standards::non_fungible_token::core::{
    NonFungibleTokenCore, NonFungibleTokenResolver,
};
use near_contract_standards::non_fungible_token::enumeration::NonFungibleTokenEnumeration;
use near_contract_standards::non_fungible_token::events::NftBurn;
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::{NonFungibleToken, Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, UnorderedMap};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, near_bindgen, require, AccountId, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue,
};
use std::collections::HashMap;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    tokens: NonFungibleToken,
    tokens_on_sale: UnorderedMap<TokenId, u128>,
    metadata: LazyOption<NFTContractMetadata>,
}

const DATA_IMAGE_SVG_NEAR_ICON: &str = r#"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 2000 2000' width='2000' height='2000'%3E%3Cg%3E%3Cg fill='%23eda735'%3E%3Cpath d='M858.44,9.87c197.07-27.87,402.41,3.5,581,91.48,203.21,99,370.36,268.93,466,473.26,86.39,182.58,114.19,391.53,81,590.48-33.18,203.07-131.84,394.65-277.46,540.37-139.73,141.09-322.78,239.19-518.1,276.43-202.08,39-416.31,14.87-603.61-70.61-208.84-94.1-382.75-262.44-483.79-467.63-94.91-190.08-125.21-410.9-88-619.72C50.36,623.84,149,435.89,293,292.55,444.26,140.59,645.84,39.49,858.44,9.87m-0.38,224.94c-176.79,32-340.56,128.34-455.25,266.31C291.76,633.21,227,803,222.39,975.38c-5.51,161.71,40.94,324.67,131.84,458.76,104,155.59,266.31,271.68,448,318.55,187.06,49.36,392.64,26.12,563.17-65.48,171.16-90.6,305.88-247.19,369.11-430,63.35-180,57.34-383.28-17.15-559-68.49-164-195.45-302.8-352.83-385.78C1211,230.06,1029.22,203.19,858.06,234.81Z'/%3E%3Cpath d='m693.54 501q101.79-.37 203.58 0c.13 101-.25 201.83.13 302.8 79 .37 157.88-.12 236.89.25q.19 63.92 0 128c-79 .25-158-.12-236.89.25q-.38 63.55 0 127.09c78.75-.12 157.51.12 236.26-.12 1.13 42.61.63 85.23.75 128-79 .62-158.13 0-237.14.37-.13 47-.13 94.1.13 141.09q252.6.37 505.08.12c.38 56.74.25 113.47.13 170.33-236.26-.12-472.65.25-708.91-.25q0-155.59 0-311.3c-32.05 0-64.11-.25-96.16-.25q-.19-63.92.13-127.84c31.93-.12 64-.25 95.91-.12q.38-63.73 0-127.22c-32.05.12-64-.12-96-.25-.13-42.61-.13-85.1-.13-127.72 32.05-.12 64.11-.37 96.16-.37.33-100.91.08-201.86.08-302.86'/%3E%3C/g%3E%3Cpath d='m858.06 234.81c171.16-31.62 353-4.75 506.46 77.61 157.38 83 284.34 221.82 352.83 385.78 74.5 175.71 80.51 379 17.15 559-63.23 182.83-197.95 339.42-369.11 430-170.53 91.6-376.12 114.85-563.17 65.48-181.67-46.86-343.94-163-448-318.55-90.88-134.13-137.34-297.04-131.83-458.75 4.61-172.38 69.37-342.17 180.42-474.26 114.69-138 278.46-234.32 455.25-266.31m-164.52 266.19c0 101 .25 202-.13 302.8-32.05 0-64.11.25-96.16.37 0 42.61 0 85.1.13 127.72 32.05.12 64 .37 96 .25q.38 63.55 0 127.22c-31.93-.12-64 0-95.91.12q-.38 63.92-.13 127.84c32.05 0 64.11.25 96.16.25q.19 155.59 0 311.3c236.26.5 472.65.12 708.91.25.13-56.86.25-113.6-.13-170.33q-252.6 0-505.08-.12c-.25-47-.25-94.1-.13-141.09 79-.37 158.13.25 237.14-.37-.13-42.74.38-85.35-.75-128-78.75.25-157.51 0-236.26.12q-.38-63.55 0-127.09c78.88-.37 157.88 0 236.89-.25q.19-64.11 0-128c-79-.37-157.88.12-236.89-.25-.38-101 0-201.83-.13-302.8q-101.74-.32-203.53.06' fill='%23231f20'/%3E%3C/g%3E%3C/svg%3E"#;

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
    TokensOnSale,
}

#[near_bindgen]
impl Contract {
    /// Initializes the contract owned by `owner_id` with
    /// default metadata (for example purposes only).
    #[init]
    pub fn new_default_meta(owner_id: AccountId) -> Self {
        Self::new(
            owner_id,
            NFTContractMetadata {
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Q-bit LOL-market".to_string(),
                symbol: "ЛОЛ".to_string(),
                icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
                base_uri: None,
                reference: None,
                reference_hash: None,
            },
        )
    }

    #[init]
    pub fn new(owner_id: AccountId, metadata: NFTContractMetadata) -> Self {
        require!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        Self {
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            tokens_on_sale: UnorderedMap::new(StorageKey::TokensOnSale),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        }
    }

    /// Mint a new token with ID=`token_id` belonging to `token_owner_id`.
    ///
    /// Since this example implements metadata, it also requires per-token metadata to be provided
    /// in this call. `self.tokens.mint` will also require it to be Some, since
    /// `StorageKey::TokenMetadata` was provided at initialization.
    ///
    /// `self.tokens.mint` will enforce `predecessor_account_id` to equal the `owner_id` given in
    /// initialization call to `new`.
    #[payable]
    pub fn nft_mint(
        &mut self,
        token_id: TokenId,
        token_owner_id: AccountId,
        token_metadata: TokenMetadata,
    ) -> Token {
        assert_eq!(
            env::predecessor_account_id(),
            self.tokens.owner_id,
            "Unauthorized"
        );
        self.tokens
            .internal_mint(token_id, token_owner_id, Some(token_metadata))
    }

    #[payable]
    pub fn nft_burn(&mut self, token_id: TokenId) {
        near_sdk::assert_one_yocto();

        // Remember current storage usage if refund_id is Some
        let initial_storage_usage = env::storage_usage();

        let Some(token_owner_id) = self.tokens.owner_by_id.remove(&token_id) else {
            env::panic_str("Token not found");
        };

        require!(
            env::predecessor_account_id() == self.tokens.owner_id
                || env::predecessor_account_id() == token_owner_id,
            "Unauthorized"
        );

        self.tokens_on_sale.remove(&token_id);

        let Some(token_metadata) = self.tokens
            .token_metadata_by_id
            .as_mut()
            .and_then(|by_id| by_id.remove(&token_id)) else {
                env::panic_str("Internal error: token_id not in token_metadata_by_id");
            };

        // Enumeration extension: Record tokens_per_owner for use with enumeration view methods.
        if let Some(tokens_per_owner) = &mut self.tokens.tokens_per_owner {
            let Some(mut token_ids) = tokens_per_owner.get(&token_owner_id) else {
                env::panic_str("Internal error: owner_id not in tokens_per_owner");
            };
            token_ids.remove(&token_id);
            tokens_per_owner.insert(&token_owner_id, &token_ids);
        }

        if let Some(reclaimed_storage) = initial_storage_usage.checked_sub(env::storage_usage()) {
            let refund = env::storage_byte_cost() * near_sdk::Balance::from(reclaimed_storage);
            Promise::new(token_metadata.extra.unwrap().parse().unwrap()).transfer(refund);
        }
        NftBurn {
            owner_id: &token_owner_id,
            token_ids: &[&token_id],
            memo: None,
            authorized_id: None,
        }
        .emit();
    }

    pub fn nft_tokens_on_sale(&self) -> std::collections::HashMap<TokenId, U128> {
        self.tokens_on_sale
            .iter()
            .map(|(token_id, price)| (token_id, price.into()))
            .collect()
    }

    #[payable]
    pub fn nft_put_on_sale(&mut self, token_id: TokenId, price: U128) {
        let token_owner_id = self.tokens.owner_by_id.get(&token_id).unwrap();
        require!(
            env::predecessor_account_id() == token_owner_id,
            "Unauthorized"
        );
        self.tokens_on_sale.insert(&token_id, &price.into());
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
enum MarketAction {
    Mint {
        title: String,       // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
        description: String, // free-form description
        media: String, // URL to associated media, preferably to decentralized, content-addressed storage
    },
    Buy(TokenId),
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    #[payable]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        require!(near_sdk::env::predecessor_account_id().as_str() == "lolcoin.qbit.near");
        let deposit: u128 = amount.into();
        require!(
            deposit >= 100,
            "At least 1.00 LOL is required to mint or buy a token"
        );
        let market_action = near_sdk::serde_json::from_str::<MarketAction>(&msg).unwrap();
        match market_action {
            MarketAction::Mint {
                title,
                description,
                media,
            } => {
                let extra = Some(sender_id.to_string());
                self.tokens.internal_mint(
                    TokenId::from(self.tokens.owner_by_id.len().to_string()),
                    sender_id,
                    Some(TokenMetadata {
                        title: Some(title),
                        description: Some(description),
                        media: Some(media),
                        extra,
                        ..Default::default()
                    }),
                );
                PromiseOrValue::Value(U128::from(0))
            }
            MarketAction::Buy(token_id) => {
                let Some(token_price) = self
                    .tokens_on_sale
                    .remove(&token_id) else {
                        near_sdk::env::panic_str("Token is not for sale");
                    };
                require!(deposit >= token_price, "Not enough funds");
                let token_current_owner = self.tokens.owner_by_id.get(&token_id).unwrap();
                self.tokens.internal_transfer(
                    &token_current_owner,
                    &sender_id,
                    &token_id,
                    None,
                    None,
                );
                near_contract_standards::fungible_token::core::ext_ft_core::ext(
                    near_sdk::env::predecessor_account_id(),
                )
                .ft_transfer(token_current_owner, token_price.into(), None);
                let refund = deposit - token_price;
                PromiseOrValue::Value(U128::from(refund))
            }
        }
    }
}

#[near_bindgen]
impl NonFungibleTokenCore for Contract {
    #[payable]
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) {
        self.tokens_on_sale.remove(&token_id);
        self.tokens
            .nft_transfer(receiver_id, token_id, approval_id, memo);
    }

    #[payable]
    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<bool> {
        self.tokens_on_sale.remove(&token_id);
        self.tokens
            .nft_transfer_call(receiver_id, token_id, approval_id, memo, msg)
    }

    fn nft_token(&self, token_id: TokenId) -> Option<Token> {
        self.tokens.nft_token(token_id)
    }
}

#[near_bindgen]
impl NonFungibleTokenResolver for Contract {
    #[private]
    fn nft_resolve_transfer(
        &mut self,
        previous_owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
        approved_account_ids: Option<HashMap<AccountId, u64>>,
    ) -> bool {
        self.tokens.nft_resolve_transfer(
            previous_owner_id,
            receiver_id,
            token_id,
            approved_account_ids,
        )
    }
}

#[near_bindgen]
impl NonFungibleTokenApproval for Contract {
    #[payable]
    fn nft_approve(
        &mut self,
        token_id: TokenId,
        account_id: AccountId,
        msg: Option<String>,
    ) -> Option<Promise> {
        self.tokens.nft_approve(token_id, account_id, msg)
    }

    #[payable]
    fn nft_revoke(&mut self, token_id: TokenId, account_id: AccountId) {
        self.tokens.nft_revoke(token_id, account_id);
    }

    #[payable]
    fn nft_revoke_all(&mut self, token_id: TokenId) {
        self.tokens.nft_revoke_all(token_id);
    }

    fn nft_is_approved(
        &self,
        token_id: TokenId,
        approved_account_id: AccountId,
        approval_id: Option<u64>,
    ) -> bool {
        self.tokens
            .nft_is_approved(token_id, approved_account_id, approval_id)
    }
}

#[near_bindgen]
impl NonFungibleTokenEnumeration for Contract {
    fn nft_total_supply(&self) -> U128 {
        self.tokens.nft_total_supply()
    }

    fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u64>) -> Vec<Token> {
        self.tokens.nft_tokens(from_index, limit)
    }

    fn nft_supply_for_owner(&self, account_id: AccountId) -> U128 {
        self.tokens.nft_supply_for_owner(account_id)
    }

    fn nft_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>,
        limit: Option<u64>,
    ) -> Vec<Token> {
        self.tokens
            .nft_tokens_for_owner(account_id, from_index, limit)
    }
}

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Contract {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use std::collections::HashMap;

    use super::*;

    const MINT_STORAGE_COST: u128 = 5870000000000000000000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    fn sample_token_metadata() -> TokenMetadata {
        TokenMetadata {
            title: Some("Olympus Mons".into()),
            description: Some("The tallest mountain in the charted solar system".into()),
            media: None,
            media_hash: None,
            copies: Some(1u64),
            issued_at: None,
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: None,
            reference: None,
            reference_hash: None,
        }
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new_default_meta(accounts(1).into());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.nft_token("1".to_string()), None);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_mint() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());

        let token_id = "0".to_string();
        let token = contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());
        assert_eq!(token.token_id, token_id);
        assert_eq!(token.owner_id, accounts(0));
        assert_eq!(token.metadata.unwrap(), sample_token_metadata());
        assert_eq!(token.approved_account_ids.unwrap(), HashMap::new());
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "0".to_string();
        contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_transfer(accounts(1), token_id.clone(), None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        if let Some(token) = contract.nft_token(token_id.clone()) {
            assert_eq!(token.token_id, token_id);
            assert_eq!(token.owner_id, accounts(1));
            assert_eq!(token.metadata.unwrap(), sample_token_metadata());
            assert_eq!(token.approved_account_ids.unwrap(), HashMap::new());
        } else {
            panic!("token not correctly created, or not found by nft_token");
        }
    }

    #[test]
    fn test_approve() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "0".to_string();
        contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

        // alice approves bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(150000000000000000000)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_approve(token_id.clone(), accounts(1), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert!(contract.nft_is_approved(token_id.clone(), accounts(1), Some(1)));
    }

    #[test]
    fn test_revoke() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "0".to_string();
        contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

        // alice approves bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(150000000000000000000)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_approve(token_id.clone(), accounts(1), None);

        // alice revokes bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_revoke(token_id.clone(), accounts(1));
        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert!(!contract.nft_is_approved(token_id.clone(), accounts(1), None));
    }

    #[test]
    fn test_revoke_all() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(0).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_STORAGE_COST)
            .predecessor_account_id(accounts(0))
            .build());
        let token_id = "0".to_string();
        contract.nft_mint(token_id.clone(), accounts(0), sample_token_metadata());

        // alice approves bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(150000000000000000000)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_approve(token_id.clone(), accounts(1), None);

        // alice revokes bob
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(0))
            .build());
        contract.nft_revoke_all(token_id.clone());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert!(!contract.nft_is_approved(token_id.clone(), accounts(1), Some(1)));
    }
}
