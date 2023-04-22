use super::PaymentContract;
use crate::constants::NANOS_IN_DAY;
use crate::contract::PaymentContractExt;
use crate::public::payment_info::PaymentInfo;
use crate::public::payment_receipt::PaymentReceipt;
use crate::public::StorageKey;
use crate::{
    error::{require, ContractError},
    Result,
};
use near_sdk::{
    env,
    json_types::{U128, U64},
    near_bindgen,
    store::UnorderedSet,
    AccountId,
};

#[near_bindgen]
impl PaymentContract {
    #[payable]
    #[handle_result]
    pub fn create_payment(
        &mut self,
        days_period_duration: U64,
        payment_amount: U128,
        receiver: AccountId,
    ) -> Result<u64> {
        let caller = env::predecessor_account_id();
        let attached_deposit = env::attached_deposit();

        let days_period_duration = days_period_duration.0;
        let payment_amount = payment_amount.0;

        require(
            attached_deposit > 0 && payment_amount > 0 && days_period_duration > 0,
            ContractError::ZeroPaymentCreationParams(
                attached_deposit,
                payment_amount,
                days_period_duration,
            ),
        )?;

        require(
            attached_deposit
                .checked_rem(payment_amount)
                .filter(|res| *res == 0)
                .is_some(),
            ContractError::IncorrectAmountRelatedParams(attached_deposit, payment_amount),
        )?; // this check will guarantee that at list one period payment could be made
            // also it checks that payment amount could be an equal part of the total amount

        let payment_id = self.payment_id_counter;
        self.payment_id_counter += 1;

        let issuer_id_store = match self.issuer_ledger.get_mut(&caller) {
            Some(value) => value,
            None => {
                self.issuer_ledger.insert(
                    caller.clone(),
                    UnorderedSet::new(StorageKey::IssuerLedgerRecord {
                        user: caller.clone(),
                    }),
                );

                self.issuer_ledger.get_mut(&caller).unwrap()
            }
        };

        require(
            issuer_id_store.insert(payment_id),
            ContractError::PaymentIdAlreadyExists(payment_id),
        )?;

        let receiver_id_store = match self.receiver_ledger.get_mut(&receiver) {
            Some(value) => value,
            None => {
                self.receiver_ledger.insert(
                    receiver.clone(),
                    UnorderedSet::new(StorageKey::ReceiverLedgerRecord {
                        user: receiver.clone(),
                    }),
                );

                self.receiver_ledger.get_mut(&receiver).unwrap()
            }
        };

        require(
            receiver_id_store.insert(payment_id),
            ContractError::PaymentIdAlreadyExists(payment_id),
        )?;

        require(
            self.payment_info_ledger
                .insert(
                    payment_id,
                    PaymentReceipt::create_payment_receipt(
                        PaymentInfo::new(
                            days_period_duration
                                .checked_mul(NANOS_IN_DAY)
                                .ok_or_else(|| {
                                    ContractError::InternalCalculationError(payment_id)
                                })?,
                            payment_amount,
                            attached_deposit,
                        ),
                        caller,
                        receiver,
                    ),
                )
                .is_none(),
            ContractError::PaymentIdAlreadyExists(payment_id),
        )?;

        Ok(payment_id)
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::{store::UnorderedMap, testing_env};

    use crate::contract::general_impl::tests::{get_context, issuer_acc, receiver_acc};

    use super::*;

    #[test]
    fn test_create_payment() {
        let context = get_context(issuer_acc(), 100);
        testing_env!(context.clone());

        let mut contract = PaymentContract {
            issuer_ledger: UnorderedMap::new(b"issuer_ledger".to_vec()),
            receiver_ledger: UnorderedMap::new(b"receiver_ledger".to_vec()),
            payment_info_ledger: UnorderedMap::new(b"payment_info_ledger".to_vec()),
            payment_id_counter: 0,
        };

        let payment_id = contract
            .create_payment(U64(30), U128(10), receiver_acc())
            .unwrap();

        assert_eq!(payment_id, 0);

        let payment_receipt = contract
            .payment_info_ledger
            .get(&payment_id)
            .unwrap()
            .into_current();

        assert_eq!(
            payment_receipt.payment_info.period_duration,
            30 * NANOS_IN_DAY
        );
        assert_eq!(payment_receipt.payment_info.payment_amount, 10);
        assert_eq!(payment_receipt.payment_info.total_amount, 100);
        assert_eq!(payment_receipt.payment_info.initiale_date, None);
        assert_eq!(payment_receipt.payment_info.last_payment_date, None);

        let issuer_ledger = contract.issuer_ledger.get(&issuer_acc()).unwrap();

        assert!(issuer_ledger.contains(&0));

        let receiver_ledger = contract.receiver_ledger.get(&receiver_acc()).unwrap();

        assert!(receiver_ledger.contains(&0));
    }

    #[test]
    fn create_payment_with_zero_params_should_fail() {
        let mut contract = PaymentContract {
            issuer_ledger: UnorderedMap::new(b"i".to_vec()),
            receiver_ledger: UnorderedMap::new(b"r".to_vec()),
            payment_info_ledger: UnorderedMap::new(b"p".to_vec()),
            payment_id_counter: 0,
        };

        let days_period_duration = U64(0);
        let payment_amount = U128(0);

        let context = get_context(issuer_acc(), 100);
        testing_env!(context.clone());

        assert_eq!(
            contract.create_payment(days_period_duration, payment_amount, receiver_acc()),
            Err(ContractError::ZeroPaymentCreationParams(100, 0, 0))
        );
    }

    #[test]
    fn create_payment_with_incorrect_params_should_fail() {
        let mut contract = PaymentContract {
            issuer_ledger: UnorderedMap::new(b"i".to_vec()),
            receiver_ledger: UnorderedMap::new(b"r".to_vec()),
            payment_info_ledger: UnorderedMap::new(b"p".to_vec()),
            payment_id_counter: 0,
        };

        let days_period_duration = U64(7);
        let payment_amount = U128(99);

        let context = get_context(issuer_acc(), 100);
        testing_env!(context.clone());

        assert_eq!(
            contract.create_payment(days_period_duration, payment_amount, receiver_acc()),
            Err(ContractError::IncorrectAmountRelatedParams(100, 99))
        );
    }
}
