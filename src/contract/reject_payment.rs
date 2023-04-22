use super::PaymentContract;
use crate::contract::PaymentContractExt;
use crate::error::ContractError;
use crate::public::payment_info::PaymentStatus;
use crate::public::PaymentRole;
use crate::Result;
use near_sdk::{env, json_types::U64, near_bindgen};
use near_sdk::{AccountId, Promise};

#[derive(PartialEq, Debug)]
struct RepaymentInfo {
    pub issuer_data: (AccountId, u128),
    pub receiver_data: (AccountId, u128),
}

impl RepaymentInfo {
    pub fn new(issuer: AccountId, receiver: AccountId) -> Self {
        RepaymentInfo {
            issuer_data: (issuer, 0),
            receiver_data: (receiver, 0),
        }
    }
}

#[near_bindgen]
impl PaymentContract {
    #[handle_result]
    fn check_role_exist(
        &self,
        caller: &AccountId,
        payment_id: u64,
        role: PaymentRole,
    ) -> Result<()> {
        match role {
            PaymentRole::Issuer => self.check_issue_payment_id(&caller, payment_id),
            PaymentRole::Receiver => self.check_reciever_payment_id(&caller, payment_id),
        }
    }

    #[handle_result]
    fn reject_payment_receipt_impl(&mut self, payment_id: u64) -> Result<RepaymentInfo> {
        let payment_receipt = self
            .payment_info_ledger
            .get_mut(&payment_id)
            .ok_or_else(|| ContractError::PaymentIdNotExist(payment_id))?
            .into_current_mut();

        let payment_info = &mut payment_receipt.payment_info;

        let payment_status = payment_info.calculate_payment_status(payment_id)?;

        let issuer = payment_receipt.issuer.clone();
        let receiver = payment_receipt.receiver.clone();

        let mut repayment_info = RepaymentInfo::new(issuer.clone(), receiver.clone());

        match payment_status {
            PaymentStatus::Absent => {
                let remainder_amount = payment_info.calculate_remainder_amount(payment_id)?;

                repayment_info.issuer_data.1 = remainder_amount;
            }
            PaymentStatus::PaymentReady(amount) => {
                repayment_info.receiver_data.1 = amount;
                repayment_info.issuer_data.1 = payment_info
                    .total_amount
                    .checked_sub(amount)
                    .ok_or_else(|| ContractError::InternalCalculationError(payment_id))?;
            }
            PaymentStatus::FinalPayment(amount) => {
                repayment_info.receiver_data.1 = amount;
            }
        }

        self.remove_payment_related_data(&issuer, &receiver, payment_id)?;

        Ok(repayment_info)
    }

    #[handle_result]
    pub fn reject_payment_receipt(&mut self, payment_id: U64, role: PaymentRole) -> Result<()> {
        let caller = env::predecessor_account_id();
        let payment_id = payment_id.0;

        self.check_role_exist(&caller, payment_id, role)?;

        // TODO Particular transfers could possibly fail because the transfee account could be deleted, need to be somehow handled
        let RepaymentInfo {
            issuer_data,
            receiver_data,
        } = self.reject_payment_receipt_impl(payment_id)?;

        if issuer_data.1 > 0 {
            Promise::new(issuer_data.0).transfer(issuer_data.1);
        }

        if receiver_data.1 > 0 {
            Promise::new(receiver_data.0).transfer(receiver_data.1);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        constants::NANOS_IN_DAY,
        contract::general_impl::tests::{
            check_all_data_removed, contract_acc, create_payment, get_context, issuer_acc,
            receiver_acc, set_block_timestamp,
        },
        public::ProcessStatus,
    };

    use super::*;
    use near_sdk::testing_env;

    #[test]
    fn test_check_roles_exist() {
        // set contract as an account of contract
        let mut context = get_context(contract_acc(), 1);
        context.current_account_id = contract_acc();
        testing_env!(context.clone());

        let mut contract = PaymentContract::new().unwrap();

        // create a payment
        let payment_id = create_payment(&mut contract, 10, 1);

        // check issuer exists
        let result = contract.check_role_exist(&issuer_acc(), payment_id, PaymentRole::Issuer);
        assert!(result.is_ok());

        // check receiver exists
        let result = contract.check_role_exist(&receiver_acc(), payment_id, PaymentRole::Receiver);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_roles_not_exist() {
        // set contract as an account of contract
        let mut context = get_context(contract_acc(), 1);
        context.current_account_id = contract_acc();
        testing_env!(context.clone());

        let contract = PaymentContract::new().unwrap();
        // check issuer exists
        let result = contract.check_role_exist(&issuer_acc(), 1, PaymentRole::Issuer);
        assert_eq!(
            result,
            Err(ContractError::IssuerAccountNotExist(issuer_acc()))
        );

        // check receiver exists
        let result = contract.check_role_exist(&receiver_acc(), 1, PaymentRole::Receiver);
        assert_eq!(
            result,
            Err(ContractError::ReceiverAccountNotExist(receiver_acc()))
        );
    }

    #[test]
    fn test_check_roles_exist_but_payment_id_wrong() {
        // set contract as an account of contract
        let mut context = get_context(contract_acc(), 1);
        context.current_account_id = contract_acc();
        testing_env!(context.clone());

        let mut contract = PaymentContract::new().unwrap();

        // create a payment
        let payment_id = create_payment(&mut contract, 10, 1);

        // check issuer exists
        let result = contract.check_role_exist(&issuer_acc(), payment_id + 1, PaymentRole::Issuer);
        assert_eq!(
            result,
            Err(ContractError::PaymentIdNotExist(payment_id + 1))
        );

        // check receiver exists
        let result =
            contract.check_role_exist(&receiver_acc(), payment_id + 1, PaymentRole::Receiver);
        assert_eq!(
            result,
            Err(ContractError::PaymentIdNotExist(payment_id + 1))
        );
    }

    #[test]
    fn test_reject_payment_receipt_absent() {
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

        // reject payment when payment when payment is absent
        let result = contract.reject_payment_receipt_impl(payment_id).unwrap();
        assert_eq!(result.issuer_data.0, issuer_acc());
        assert_eq!(result.issuer_data.1, 10);
        assert_eq!(result.receiver_data.0, receiver_acc());
        assert_eq!(result.receiver_data.1, 0);

        // check that the payment has been removed from all storages
        check_all_data_removed(&contract, payment_id);
    }

    #[test]
    fn test_reject_payment_receipt_payment_ready() {
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
        // reject payment when payment when payment is ready
        let result = contract.reject_payment_receipt_impl(payment_id).unwrap();
        assert_eq!(result.issuer_data.0, issuer_acc());
        assert_eq!(result.issuer_data.1, 5);
        assert_eq!(result.receiver_data.0, receiver_acc());
        assert_eq!(result.receiver_data.1, 5);

        // check that the payment has been removed from all storages
        check_all_data_removed(&contract, payment_id);
    }

    #[test]
    fn test_reject_payment_receipt_final_payment() {
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
        // reject payment when payment when payment is final
        let result = contract.reject_payment_receipt_impl(payment_id).unwrap();
        assert_eq!(result.issuer_data.0, issuer_acc());
        assert_eq!(result.issuer_data.1, 0);
        assert_eq!(result.receiver_data.0, receiver_acc());
        assert_eq!(result.receiver_data.1, 10);

        // check that the payment has been removed from all storages
        check_all_data_removed(&contract, payment_id);
    }

    #[test]
    fn test_reject_payment_receipt_not_exist() {
        // set contract as an account of contract
        let mut context = get_context(contract_acc(), 1);
        context.current_account_id = contract_acc();
        testing_env!(context.clone());

        let mut contract = PaymentContract::new().unwrap();
        // reject payment when payment when payment is final
        let result = contract.reject_payment_receipt_impl(1);
        assert_eq!(result, Err(ContractError::PaymentIdNotExist(1)));
    }
}
