use super::PaymentContract;
use crate::contract::PaymentContractExt;
use crate::error::ContractError;
use crate::public::payment_info::PaymentStatus;
use crate::Result;
use near_sdk::{env, json_types::U64, near_bindgen};
use near_sdk::{AccountId, Promise};

#[near_bindgen]
impl PaymentContract {
    #[handle_result]
    fn claim_payment_impl(&mut self, caller: &AccountId, payment_id: u64) -> Result<u128> {
        self.check_reciever_payment_id(&caller, payment_id)?;

        let payment_receipt = self
            .payment_info_ledger
            .get_mut(&payment_id)
            .ok_or_else(|| ContractError::PaymentIdNotExist(payment_id))?
            .into_current_mut();

        let payment_info = &mut payment_receipt.payment_info;

        let payment_status = payment_info.calculate_payment_status(payment_id)?;

        match payment_status {
            PaymentStatus::Absent => Ok(0), // nothing is required to be done in this case
            PaymentStatus::PaymentReady(amount) => {
                payment_info.last_payment_date = env::block_timestamp().into();

                Ok(amount)
            }
            PaymentStatus::FinalPayment(amount) => {
                let issuer = payment_receipt.issuer.clone();
                self.remove_payment_related_data(&issuer, &caller, payment_id)?;

                Ok(amount)
            }
        }
    }

    #[handle_result]
    pub fn claim_payment(&mut self, payment_id: U64) -> Result<()> {
        let caller = env::predecessor_account_id();

        let amount = self.claim_payment_impl(&caller, payment_id.0)?;

        if amount > 0 {
            // This case could not fail because we are paying back to the predecessor
            Promise::new(caller).transfer(amount);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        constants::NANOS_IN_DAY,
        contract::general_impl::tests::{
            check_all_data_removed, contract_acc, create_payment, get_context, receiver_acc,
            set_block_timestamp,
        },
        public::ProcessStatus,
    };

    use super::*;
    use near_sdk::testing_env;

    #[test]
    fn test_claim_payment_absent() {
        // set contract as an account of contract
        let mut context = get_context(contract_acc(), 1);
        context.current_account_id = contract_acc();
        testing_env!(context.clone());

        let mut contract = PaymentContract::new().unwrap();

        // create a payment
        let payment_id = create_payment(&mut contract, 10, 1);

        // set caller to receiver
        let mut context = get_context(receiver_acc(), 0);
        context.block_timestamp = 1;
        testing_env!(context.clone());

        // approve the payment
        contract
            .process_pending_payment(ProcessStatus::Approve(U64(payment_id)))
            .unwrap();

        set_block_timestamp(NANOS_IN_DAY / 2);
        // claim payment when payment status is absent
        let result = contract.claim_payment_impl(&receiver_acc(), payment_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_claim_multiple_payment_ready() {
        // set contract as an account of contract
        let mut context = get_context(contract_acc(), 1);
        context.current_account_id = contract_acc();
        testing_env!(context.clone());

        let mut contract = PaymentContract::new().unwrap();

        // create a payment
        let payment_id = create_payment(&mut contract, 10, 1);

        // set caller to receiver
        let mut context = get_context(receiver_acc(), 0);
        context.block_timestamp = 1;
        testing_env!(context.clone());

        // approve the payment
        contract
            .process_pending_payment(ProcessStatus::Approve(U64(payment_id)))
            .unwrap();

        // we set to the fifth day(period is one day, period_amount is 1token, so we will claim 5 tokens)
        set_block_timestamp(NANOS_IN_DAY * 5 + 1);
        // claim payment when payment status is absent
        let result = contract.claim_payment_impl(&receiver_acc(), payment_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);

        // check that last payment date was overwritten
        assert_eq!(
            contract
                .payment_info_ledger
                .get(&payment_id)
                .unwrap()
                .into_current()
                .payment_info
                .last_payment_date
                .unwrap(),
            NANOS_IN_DAY * 5 + 1
        );

        // we set to the fifth day(period is one day, period_amount is 1token, so we will claim 5 tokens)
        set_block_timestamp(NANOS_IN_DAY * 6 + 1);
        let result = contract.claim_payment_impl(&receiver_acc(), payment_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // check that last payment date was overwritten
        assert_eq!(
            contract
                .payment_info_ledger
                .get(&payment_id)
                .unwrap()
                .into_current()
                .payment_info
                .last_payment_date
                .unwrap(),
            NANOS_IN_DAY * 6 + 1
        );
    }

    #[test]
    fn test_claim_payment_final() {
        // set contract as an account of contract
        let mut context = get_context(contract_acc(), 1);
        context.current_account_id = contract_acc();
        testing_env!(context.clone());

        let mut contract = PaymentContract::new().unwrap();

        // create a payment
        let payment_id = create_payment(&mut contract, 10, 1);

        // set caller to receiver
        let mut context = get_context(receiver_acc(), 0);
        context.block_timestamp = 1;
        testing_env!(context.clone());

        // approve the payment
        contract
            .process_pending_payment(ProcessStatus::Approve(U64(payment_id)))
            .unwrap();

        // we set to the final 10th day after the start day
        set_block_timestamp(NANOS_IN_DAY * 10 + 1);
        let result = contract.claim_payment_impl(&receiver_acc(), payment_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 10);

        // check that the payment has been removed from all storages
        check_all_data_removed(&contract, payment_id);
    }
}
