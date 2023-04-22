use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
};
use serde::Serialize;

use crate::error::ContractError;

#[derive(PartialEq, Debug)]
pub(crate) enum PaymentStatus {
    Absent,
    PaymentReady(u128),
    FinalPayment(u128),
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Clone)]
pub struct PaymentInfo {
    pub initiale_date: Option<u64>,
    pub period_duration: u64,
    pub payment_amount: u128,
    pub total_amount: u128,
    pub last_payment_date: Option<u64>,
}

impl PaymentInfo {
    pub fn new(period_duration: u64, payment_amount: u128, total_amount: u128) -> Self {
        Self {
            initiale_date: None,
            period_duration,
            payment_amount,
            total_amount,
            last_payment_date: None,
        }
    }

    fn calculate_payment_status_impl(
        &mut self,
        payment_id: u64,
        current_time: u64,
    ) -> Result<PaymentStatus, ContractError> {
        match self.initiale_date {
            Some(initiale_date) => {
                let last_payment_received = self.last_payment_date.unwrap_or(initiale_date);

                let mut number_of_available_payments = current_time
                    .checked_sub(last_payment_received)
                    .and_then(|diff| diff.checked_div(self.period_duration))
                    .unwrap_or(0);

                let number_of_made_payments = last_payment_received
                    .checked_sub(initiale_date)
                    .and_then(|diff| diff.checked_div(self.period_duration))
                    .unwrap_or(0);

                let max_payments_number = self
                    .total_amount
                    .checked_div(self.payment_amount)
                    .ok_or_else(|| ContractError::InternalCalculationError(payment_id))?
                    as u64;

                if number_of_available_payments + number_of_made_payments > max_payments_number {
                    number_of_available_payments = max_payments_number
                        .checked_sub(number_of_made_payments)
                        .unwrap_or(0);
                }

                let end_date = initiale_date
                    .checked_add(
                        max_payments_number
                            .checked_mul(self.period_duration)
                            .ok_or_else(|| ContractError::InternalCalculationError(payment_id))?,
                    )
                    .ok_or_else(|| ContractError::InternalCalculationError(payment_id))?;

                let amount = self
                    .payment_amount
                    .checked_mul(number_of_available_payments as u128)
                    .ok_or_else(|| ContractError::InternalCalculationError(payment_id))?;

                if amount == 0 {
                    Ok(PaymentStatus::Absent)
                } else if current_time >= end_date {
                    Ok(PaymentStatus::FinalPayment(amount))
                } else {
                    Ok(PaymentStatus::PaymentReady(amount))
                }
            }
            None => Err(ContractError::PaymentReceiptNotConfirmed(payment_id)),
        }
    }

    pub(crate) fn calculate_payment_status(
        &mut self,
        payment_id: u64,
    ) -> Result<PaymentStatus, ContractError> {
        let current_time = env::block_timestamp();

        self.calculate_payment_status_impl(payment_id, current_time)
    }

    pub(crate) fn calculate_remainder_amount(
        &self,
        payment_id: u64,
    ) -> Result<u128, ContractError> {
        match self.initiale_date {
            Some(intiale_date) => match self.last_payment_date {
                Some(last_payment_date) => {
                    let number_of_received_payments = last_payment_date
                        .checked_sub(intiale_date)
                        .map(|value| value.checked_div(self.period_duration))
                        .flatten()
                        .ok_or_else(|| ContractError::InternalCalculationError(payment_id))?;

                    let total_payed = self
                        .payment_amount
                        .checked_mul(number_of_received_payments as u128)
                        .ok_or_else(|| ContractError::InternalCalculationError(payment_id))?;

                    self.total_amount
                        .checked_sub(total_payed)
                        .ok_or_else(|| ContractError::InternalCalculationError(payment_id))
                }
                None => Ok(self.total_amount),
            },
            None => Ok(self.total_amount),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_payment_status_no_initial_date() {
        let mut payment_info = PaymentInfo::new(60, 100, 500);

        assert_eq!(
            payment_info.calculate_payment_status_impl(0, 0),
            Err(ContractError::PaymentReceiptNotConfirmed(0))
        );
    }

    #[test]
    fn test_calculate_payment_status_absent() {
        let mut payment_info = PaymentInfo::new(60, 100, 500);
        payment_info.initiale_date = Some(0);

        assert_eq!(
            payment_info.calculate_payment_status(0),
            Ok(PaymentStatus::Absent)
        );
    }

    #[test]
    fn test_calculate_payment_status_absent_after_some_period() {
        let mut payment_info = PaymentInfo::new(60, 100, 500);
        payment_info.initiale_date = Some(0);

        assert_eq!(
            payment_info.calculate_payment_status_impl(0, 59),
            Ok(PaymentStatus::Absent)
        );
    }

    #[test]
    fn test_calculate_payment_status_absent_after_some_period_and_after_payment() {
        let mut payment_info = PaymentInfo::new(60, 100, 500);
        payment_info.initiale_date = Some(0);
        payment_info.last_payment_date = Some(70);

        assert_eq!(
            payment_info.calculate_payment_status_impl(0, 80),
            Ok(PaymentStatus::Absent)
        );
    }

    #[test]
    fn test_calculate_payment_status_final_payment() {
        let mut payment_info = PaymentInfo::new(60, 100, 500);
        payment_info.initiale_date = Some(0);
        payment_info.last_payment_date = Some(120);

        assert_eq!(
            payment_info.calculate_payment_status_impl(0, 500),
            Ok(PaymentStatus::FinalPayment(300))
        );
    }

    #[test]
    fn test_calculate_payment_status_final_payment_for_last_period() {
        let mut payment_info = PaymentInfo::new(60, 100, 500);
        payment_info.initiale_date = Some(0);
        payment_info.last_payment_date = Some(240);

        assert_eq!(
            payment_info.calculate_payment_status_impl(0, 300),
            Ok(PaymentStatus::FinalPayment(100))
        );
    }

    #[test]
    fn test_calculate_payment_status_payment_ready() {
        let mut payment_info = PaymentInfo::new(60, 100, 500);
        payment_info.initiale_date = Some(0);

        assert_eq!(
            payment_info.calculate_payment_status_impl(0, 60),
            Ok(PaymentStatus::PaymentReady(100))
        );
    }

    #[test]
    fn test_calculate_payment_status_payment_ready_after_payment() {
        let mut payment_info = PaymentInfo::new(60, 100, 500);
        payment_info.initiale_date = Some(0);
        payment_info.last_payment_date = Some(70);

        assert_eq!(
            payment_info.calculate_payment_status_impl(0, 190),
            Ok(PaymentStatus::PaymentReady(200))
        );
    }

    #[test]
    fn test_calculate_remainder_amount_no_initial_date() {
        let payment_info = PaymentInfo::new(60, 100, 500);

        assert_eq!(payment_info.calculate_remainder_amount(0), Ok(500));
    }

    #[test]
    fn test_calculate_remainder_amount_no_payments_made() {
        let mut payment_info = PaymentInfo::new(60, 100, 500);
        payment_info.initiale_date = Some(0);

        assert_eq!(payment_info.calculate_remainder_amount(0), Ok(500));
    }

    #[test]
    fn test_calculate_remainder_amount_some_payments_made() {
        let mut payment_info = PaymentInfo::new(60, 100, 500);
        payment_info.initiale_date = Some(0);
        payment_info.last_payment_date = Some(60);

        assert_eq!(payment_info.calculate_remainder_amount(0), Ok(400));
    }
}
