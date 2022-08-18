/*!
Fungible Token implementation with JSON serialization.
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
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};

mod internal;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
    reward_operators: std::collections::HashSet<AccountId>,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Reward {
    target_account_id: AccountId,
    tokens_amount: U128,
    memo: Option<String>,
}

const DATA_IMAGE_SVG_NEAR_ICON: &str = r#"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 2000 2000' width='2000' height='2000'%3E%3Cg%3E%3Cg fill='%23eda735'%3E%3Cpath d='M858.44,9.87c197.07-27.87,402.41,3.5,581,91.48,203.21,99,370.36,268.93,466,473.26,86.39,182.58,114.19,391.53,81,590.48-33.18,203.07-131.84,394.65-277.46,540.37-139.73,141.09-322.78,239.19-518.1,276.43-202.08,39-416.31,14.87-603.61-70.61-208.84-94.1-382.75-262.44-483.79-467.63-94.91-190.08-125.21-410.9-88-619.72C50.36,623.84,149,435.89,293,292.55,444.26,140.59,645.84,39.49,858.44,9.87m-0.38,224.94c-176.79,32-340.56,128.34-455.25,266.31C291.76,633.21,227,803,222.39,975.38c-5.51,161.71,40.94,324.67,131.84,458.76,104,155.59,266.31,271.68,448,318.55,187.06,49.36,392.64,26.12,563.17-65.48,171.16-90.6,305.88-247.19,369.11-430,63.35-180,57.34-383.28-17.15-559-68.49-164-195.45-302.8-352.83-385.78C1211,230.06,1029.22,203.19,858.06,234.81Z'/%3E%3Cpath d='m693.54 501q101.79-.37 203.58 0c.13 101-.25 201.83.13 302.8 79 .37 157.88-.12 236.89.25q.19 63.92 0 128c-79 .25-158-.12-236.89.25q-.38 63.55 0 127.09c78.75-.12 157.51.12 236.26-.12 1.13 42.61.63 85.23.75 128-79 .62-158.13 0-237.14.37-.13 47-.13 94.1.13 141.09q252.6.37 505.08.12c.38 56.74.25 113.47.13 170.33-236.26-.12-472.65.25-708.91-.25q0-155.59 0-311.3c-32.05 0-64.11-.25-96.16-.25q-.19-63.92.13-127.84c31.93-.12 64-.25 95.91-.12q.38-63.73 0-127.22c-32.05.12-64-.12-96-.25-.13-42.61-.13-85.1-.13-127.72 32.05-.12 64.11-.37 96.16-.37.33-100.91.08-201.86.08-302.86'/%3E%3C/g%3E%3Cpath d='m858.06 234.81c171.16-31.62 353-4.75 506.46 77.61 157.38 83 284.34 221.82 352.83 385.78 74.5 175.71 80.51 379 17.15 559-63.23 182.83-197.95 339.42-369.11 430-170.53 91.6-376.12 114.85-563.17 65.48-181.67-46.86-343.94-163-448-318.55-90.88-134.13-137.34-297.04-131.83-458.75 4.61-172.38 69.37-342.17 180.42-474.26 114.69-138 278.46-234.32 455.25-266.31m-164.52 266.19c0 101 .25 202-.13 302.8-32.05 0-64.11.25-96.16.37 0 42.61 0 85.1.13 127.72 32.05.12 64 .37 96 .25q.38 63.55 0 127.22c-31.93-.12-64 0-95.91.12q-.38 63.92-.13 127.84c32.05 0 64.11.25 96.16.25q.19 155.59 0 311.3c236.26.5 472.65.12 708.91.25.13-56.86.25-113.6-.13-170.33q-252.6 0-505.08-.12c-.25-47-.25-94.1-.13-141.09 79-.37 158.13.25 237.14-.37-.13-42.74.38-85.35-.75-128-78.75.25-157.51 0-236.26.12q-.38-63.55 0-127.09c78.88-.37 157.88 0 236.89-.25q.19-64.11 0-128c-79-.37-157.88.12-236.89-.25-.38-101 0-201.83-.13-302.8q-101.74-.32-203.53.06' fill='%23231f20'/%3E%3C/g%3E%3C/svg%3E"#;

#[near_bindgen]
impl Contract {
    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// default metadata (for example purposes only).
    #[init]
    pub fn new_default_meta(reward_operators: std::collections::HashSet<AccountId>) -> Self {
        Self::new(
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "Q-bit LOL-coin".to_string(),
                symbol: "ЛОЛ".to_string(),
                icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
                reference: None,
                reference_hash: None,
                decimals: 2,
            },
            reward_operators,
        )
    }

    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
    #[init]
    pub fn new(
        metadata: FungibleTokenMetadata,
        reward_operators: std::collections::HashSet<AccountId>,
    ) -> Self {
        metadata.assert_valid();
        Self {
            token: FungibleToken::new(b"a".to_vec()),
            metadata: LazyOption::new(b"m".to_vec(), Some(&metadata)),
            reward_operators,
        }
    }

    pub fn reset_metadata(&mut self) {
        self.metadata.replace(&FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: "Q-bit LOL-coin".to_string(),
            symbol: "ЛОЛ".to_string(),
            icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
            reference: None,
            reference_hash: None,
            decimals: 2,
        });
    }

    pub fn reward(&mut self, rewards: Vec<Reward>, memo: Option<String>) {
        self.assert_reward_operator();
        let mut events = vec![];
        for reward in &rewards {
            if !self.token.accounts.contains_key(&reward.target_account_id) {
                self.token
                    .internal_register_account(&reward.target_account_id);
            }
            self.token
                .internal_deposit(&reward.target_account_id, reward.tokens_amount.into());
            events.push(near_contract_standards::fungible_token::events::FtMint {
                owner_id: &reward.target_account_id,
                amount: &reward.tokens_amount,
                memo: reward.memo.as_deref().or_else(|| memo.as_deref()),
            });
        }
        near_contract_standards::fungible_token::events::FtMint::emit_many(&events);
    }

    pub fn custodial_ft_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    ) {
        self.assert_reward_operator();
        self.token
            .internal_transfer(&sender_id, &receiver_id, amount.into(), memo);
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, Balance};

    use super::*;

    const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new_default_meta(accounts(1).into(), TOTAL_SUPPLY.into());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new_default_meta(accounts(2).into(), TOTAL_SUPPLY.into());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(
            contract.ft_balance_of(accounts(2)).0,
            (TOTAL_SUPPLY - transfer_amount)
        );
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
