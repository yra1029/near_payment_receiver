use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    AccountId,
};
use serde::Serialize;

use super::payment_info::PaymentInfo;

#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum PaymentReceipt {
    V1(PaymentReceiptV1),
}

pub type CurrentUserVersion = PaymentReceiptV1;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct PaymentReceiptV1 {
    pub payment_info: PaymentInfo,
    pub issuer: AccountId,
    pub receiver: AccountId,
}

impl From<PaymentReceiptV1> for PaymentReceipt {
    fn from(account: PaymentReceiptV1) -> Self {
        PaymentReceipt::V1(account)
    }
}

impl PaymentReceipt {
    pub fn create_payment_receipt(
        payment_info: PaymentInfo,
        issuer: AccountId,
        receiver: AccountId,
    ) -> PaymentReceipt {
        CurrentUserVersion {
            payment_info,
            issuer,
            receiver,
        }
        .into()
    }

    pub fn into_current(&self) -> &CurrentUserVersion {
        match self {
            Self::V1(value) => value,
        }
    }

    pub fn into_current_mut(&mut self) -> &mut CurrentUserVersion {
        match self {
            Self::V1(value) => value,
        }
    }
}
