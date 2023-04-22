use near_sdk::{
    borsh::{self, BorshSerialize},
    json_types::U64,
    AccountId, BorshStorageKey,
};
use serde::{Deserialize, Serialize};

pub mod payment_info;
pub mod payment_receipt;

#[derive(Debug, BorshStorageKey, BorshSerialize, PartialEq, Eq)]
pub enum StorageKey {
    IssuerLedger,
    ReceiverLedger,
    PaymentReceiptLedger,
    IssuerLedgerRecord { user: AccountId },
    ReceiverLedgerRecord { user: AccountId },
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum PaymentRole {
    Issuer,
    Receiver,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum ProcessStatus {
    Approve(U64),
    Reject(U64),
}
