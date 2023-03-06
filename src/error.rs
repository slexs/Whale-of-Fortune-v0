use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Invalid Token")]
    InvalidToken {},

    #[error("Invalid bet")]
    InvalidBet {}, 

    #[error("Already paid out")]
    AlreadyPaidOut {}, 
}
