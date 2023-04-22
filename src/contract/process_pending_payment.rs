use super::PaymentContract;
use crate::contract::PaymentContractExt;
use crate::error::ContractError;
use crate::public::ProcessStatus;
use crate::Result;
use near_sdk::Promise;
use near_sdk::{env, near_bindgen};

#[near_bindgen]
impl PaymentContract {
    #[handle_result]
    pub fn process_pending_payment(&mut self, process_status: ProcessStatus) -> Result<()> {
        match process_status {
            ProcessStatus::Approve(payment_id) => {
                let payment_id = payment_id.0;
                let caller = env::predecessor_account_id();

                // check whether the caller of the method has particluar record with the payment_id in the receivers list
                self.check_reciever_payment_id(&caller, payment_id)?;

                let payment_receipt = self
                    .payment_info_ledger
                    .get_mut(&payment_id)
                    .ok_or_else(|| ContractError::PaymentIdNotExist(payment_id))?
                    .into_current_mut();

                // Need to start the clock to start the payment stream
                payment_receipt.payment_info.initiale_date = Some(env::block_timestamp());
            }
            ProcessStatus::Reject(payment_id) => {
                let payment_id = payment_id.0;
                let caller = env::predecessor_account_id();

                let payment_receipt = self
                    .payment_info_ledger
                    .get_mut(&payment_id)
                    .ok_or_else(|| ContractError::PaymentIdNotExist(payment_id))?
                    .into_current();

                let issuer = payment_receipt.issuer.clone();
                let total_amount = payment_receipt.payment_info.total_amount;

                self.remove_payment_related_data(&issuer, &caller, payment_id)?;

                // making the refund
                // TODO This transaction could possibly fail because issuer account could be deleted at the time of refund, should be additionally handled,
                // this will require additional logic and fields for the smart-contract struct. As a very simple example we could have additional
                // mapping for AccountId and the Balance which would represent stuck costs because the account was deleted, but no gurantees that the same user
                // will restore the access to the account with particular name, so that this issue is rather complex from the business point of view
                Promise::new(issuer).transfer(total_amount);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::contract::general_impl::tests::{
        contract_acc, create_payment, get_context, issuer_acc, receiver_acc,
    };
    use crate::error::ContractError;

    use super::*;
    use near_sdk::json_types::U64;
    use near_sdk::testing_env;

    #[test]
    fn test_approve_payment() {
        // set contract as an account of contract
        let mut context = get_context(contract_acc(), 1);
        context.current_account_id = contract_acc();
        testing_env!(context.clone());

        let mut contract = PaymentContract::new().unwrap();

        // create a payment
        let payment_id = create_payment(&mut contract, 1, 1);

        // set caller to receiver
        let context = get_context(receiver_acc(), 0);
        testing_env!(context.clone());

        // approve the payment
        contract
            .process_pending_payment(ProcessStatus::Approve(U64(payment_id)))
            .unwrap();

        // check that the payment has been started
        let payment = contract.payment_info_ledger.get(&payment_id).unwrap();
        assert!(payment.into_current().payment_info.initiale_date.is_some());
    }

    #[test]
    fn test_reject_payment() {
        // set contract as an account of contract
        let mut context = get_context(contract_acc(), 1);
        context.current_account_id = contract_acc();
        testing_env!(context.clone());

        let mut contract = PaymentContract::new().unwrap();

        // create a payment
        let payment_id = create_payment(&mut contract, 1, 1);

        // set caller to receiver
        let context = get_context(receiver_acc(), 0);
        testing_env!(context.clone());

        // reject the payment
        contract
            .process_pending_payment(ProcessStatus::Reject(U64(payment_id)))
            .unwrap();

        // check that the payment has been removed
        let payment = contract.payment_info_ledger.get(&payment_id);
        assert!(payment.is_none());
    }

    #[test]
    fn test_reject_payment_not_receiver() {
        // set contract as an account of contract
        let mut context = get_context(contract_acc(), 1);
        context.current_account_id = contract_acc();
        testing_env!(context.clone());

        let mut contract = PaymentContract::new().unwrap();

        // create a payment
        let payment_id = create_payment(&mut contract, 1, 1);

        // set caller to issuer which is not allowed, only receiver can call this method
        let context = get_context(issuer_acc(), 0);
        testing_env!(context.clone());

        // reject the payment
        let res = contract.process_pending_payment(ProcessStatus::Reject(U64(payment_id)));

        assert_eq!(
            res,
            Err(ContractError::ReceiverAccountNotExist(issuer_acc()))
        );
    }

    #[test]
    fn test_reject_payment_twice() {
        // set contract as an account of contract
        let mut context = get_context(contract_acc(), 1);
        context.current_account_id = contract_acc();
        testing_env!(context.clone());

        let mut contract = PaymentContract::new().unwrap();

        // create a payment
        let payment_id = create_payment(&mut contract, 1, 1);

        // set caller to receiver
        let context = get_context(receiver_acc(), 0);
        testing_env!(context.clone());

        // reject the payment
        contract
            .process_pending_payment(ProcessStatus::Reject(U64(payment_id)))
            .unwrap();

        // check that the payment has been removed
        let payment = contract.payment_info_ledger.get(&payment_id);
        assert!(payment.is_none());

        // reject the payment
        let res = contract.process_pending_payment(ProcessStatus::Reject(U64(payment_id)));

        assert_eq!(res, Err(ContractError::PaymentIdNotExist(payment_id)));
    }
}
