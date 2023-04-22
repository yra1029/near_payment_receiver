pub mod constants;
pub mod contract;
pub mod error;
pub mod public;

pub type Result<T> = std::result::Result<T, error::ContractError>;
