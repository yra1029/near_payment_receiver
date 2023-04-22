pub mod claim_payment;
pub mod create_payment;
mod general_impl;
pub mod process_pending_payment;
pub mod reject_payment;

use crate::error::{require, ContractError};
use crate::public::payment_receipt::PaymentReceipt;
use crate::public::StorageKey;
use crate::Result;
use near_sdk::store::UnorderedSet;
use near_sdk::{assert_one_yocto, env};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near_bindgen,
    store::UnorderedMap,
    AccountId, PanicOnDefault,
};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct PaymentContract {
    issuer_ledger: UnorderedMap<AccountId, UnorderedSet<u64>>,
    receiver_ledger: UnorderedMap<AccountId, UnorderedSet<u64>>,
    payment_info_ledger: UnorderedMap<u64, PaymentReceipt>,
    payment_id_counter: u64,
}

#[near_bindgen]
impl PaymentContract {
    #[init]
    #[payable]
    #[handle_result]
    pub fn new() -> Result<Self> {
        assert_one_yocto(); // Required to check that initializer has a full access key
        require(
            env::predecessor_account_id() == env::current_account_id(),
            ContractError::InitializeError,
        )?;

        Ok(PaymentContract {
            issuer_ledger: UnorderedMap::new(StorageKey::IssuerLedger),
            receiver_ledger: UnorderedMap::new(StorageKey::ReceiverLedger),
            payment_info_ledger: UnorderedMap::new(StorageKey::PaymentReceiptLedger),
            payment_id_counter: 1,
        })
    }
}
