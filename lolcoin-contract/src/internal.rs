use near_sdk::env;

impl crate::Contract {
    /// Asserts that the method was called by a reward operator
    pub(crate) fn assert_reward_operator(&self) {
        assert!(
            self.reward_operators
                .contains(&env::predecessor_account_id()),
            "Can only be called by reward operators"
        );
    }
}
