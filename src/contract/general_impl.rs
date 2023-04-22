use super::PaymentContract;
use crate::contract::PaymentContractExt;
use crate::{
    error::{require, ContractError},
    Result,
};
use near_sdk::{near_bindgen, AccountId};

#[near_bindgen]
impl PaymentContract {
    #[handle_result]
    pub(crate) fn check_reciever_payment_id(
        &self,
        account_id: &AccountId,
        payment_id: u64,
    ) -> Result<()> {
        let receiver_id_store = self
            .receiver_ledger
            .get(&account_id)
            .ok_or_else(|| ContractError::ReceiverAccountNotExist(account_id.clone()))?;

        receiver_id_store
            .contains(&payment_id)
            .then_some(())
            .ok_or_else(|| ContractError::PaymentIdNotExist(payment_id))
    }

    #[handle_result]
    pub(crate) fn check_issue_payment_id(
        &self,
        account_id: &AccountId,
        payment_id: u64,
    ) -> Result<()> {
        let issue_id_store = self
            .issuer_ledger
            .get(&account_id)
            .ok_or_else(|| ContractError::IssuerAccountNotExist(account_id.clone()))?;

        issue_id_store
            .contains(&payment_id)
            .then_some(())
            .ok_or_else(|| ContractError::PaymentIdNotExist(payment_id))
    }

    #[handle_result]
    pub(crate) fn remove_payment_related_data(
        &mut self,
        issuer: &AccountId,
        receiver: &AccountId,
        payment_id: u64,
    ) -> Result<()> {
        // remove payment_id from the issue store
        require(
            self.issuer_ledger
                .get_mut(&issuer)
                .and_then(|issuer_id_store| issuer_id_store.remove(&payment_id).then_some(()))
                .is_some(),
            ContractError::IssuerAccountNotExist(issuer.clone()),
        )?;

        // remove related payment receipt
        self.payment_info_ledger
            .remove(&payment_id)
            .ok_or_else(|| ContractError::PaymentIdNotExist(payment_id))?;

        // remove payment_id from the receiver store
        require(
            self.receiver_ledger
                .get_mut(&receiver)
                .and_then(|receiver_id_store| receiver_id_store.remove(&payment_id).then_some(()))
                .is_some(),
            ContractError::ReceiverAccountNotExist(receiver.clone()),
        )
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use near_sdk::{
        json_types::{U128, U64},
        test_utils::accounts,
        testing_env, AccountId, VMContext,
    };

    use crate::contract::PaymentContract;

    pub fn contract_acc() -> AccountId {
        accounts(0)
    }

    pub fn issuer_acc() -> AccountId {
        accounts(1)
    }

    pub fn receiver_acc() -> AccountId {
        accounts(2)
    }

    pub fn check_all_data_removed(contract: &PaymentContract, payment_id: u64) {
        // check that the payment has been removed from all storages
        let payment = contract.payment_info_ledger.get(&payment_id);
        assert!(payment.is_none());

        assert!(!contract
            .issuer_ledger
            .get(&issuer_acc())
            .unwrap()
            .contains(&payment_id));

        assert!(!contract
            .receiver_ledger
            .get(&receiver_acc())
            .unwrap()
            .contains(&payment_id));
    }

    // helper function to create a payment
    pub fn create_payment(
        contract: &mut PaymentContract,
        attached_deposit: u128,
        amount: u128,
    ) -> u64 {
        let context = get_context(issuer_acc(), attached_deposit);
        testing_env!(context.clone());
        contract
            .create_payment(U64(1), U128(amount), receiver_acc())
            .unwrap()
    }

    pub fn set_block_timestamp(timestamp: u64) -> u64 {
        let mut context = get_context(issuer_acc(), 1);
        context.block_timestamp = timestamp;
        testing_env!(context.clone());
        timestamp
    }

    // Mock the context with default values
    pub fn get_context(predecessor_account_id: AccountId, attached_deposit: u128) -> VMContext {
        VMContext {
            current_account_id: contract_acc(),
            signer_account_id: predecessor_account_id.clone(),
            signer_account_pk: "ed25519:DvyD9AcDpwpRq1MY92gJwZY5W4N9UNKzSAgH7Fb5Er2w"
                .parse()
                .unwrap(),
            predecessor_account_id,
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 10u128.pow(25),
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit,
            prepaid_gas: 10u64.pow(18).into(),
            random_seed: [1; 32],
            output_data_receivers: vec![],
            epoch_height: 19,
            view_config: None,
        }
    }
}
