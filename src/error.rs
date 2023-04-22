use crate::Result;
use near_sdk::{
    borsh::{self, BorshSerialize},
    AccountId, FunctionError,
};
use serde::Deserialize;
use thiserror::Error;

pub(crate) fn require(cond: bool, err: ContractError) -> Result<()> {
    match cond {
        true => Ok(()),
        false => Err(err),
    }
}

#[derive(BorshSerialize, Debug, Error, FunctionError, Deserialize, PartialEq)]
pub enum ContractError {
    #[error("Only contract account itself is possible to initialize the contract")]
    InitializeError,
    #[error(
        "attached_deposit = {}, payment_amount = {}, days_period_duration = {} should be not 0",
        _0,
        _1,
        _2
    )]
    ZeroPaymentCreationParams(u128, u128, u64),
    #[error(
        "attached_deposit({}) should be equally devided by the payment_amount({})",
        _0,
        _1
    )]
    IncorrectAmountRelatedParams(u128, u128),
    #[error("Account {} does not have a record in receivers store", _0)]
    ReceiverAccountNotExist(AccountId),
    #[error("Account {} does not have a record in issuers store", _0)]
    IssuerAccountNotExist(AccountId),
    #[error("Payment Id {} does not exist in particular store", _0)]
    PaymentIdNotExist(u64),
    #[error("Payment receipt with the payment id {} is not confirmed", _0)]
    PaymentReceiptNotConfirmed(u64),
    #[error("Internal calculation error for payment id {}", _0)]
    InternalCalculationError(u64),
    #[error("Payment id {} already exists", _0)]
    PaymentIdAlreadyExists(u64),
}
