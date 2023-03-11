use cosmwasm_std::StdError;
use cw_utils::PaymentError;
// use serde::{Deserialize, Serialize};
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
    
    #[error("Invalid bet number")]
    InvalidBetNumber {},

    #[error("Invalid bet amount")]
    InvalidBetAmount {},

    #[error("More than one denom sent")]
    InvalidCoin{}, 

    #[error("Callback was not called by beacon, but by someone else")]
    InvalidEntropyCallback {},

    #[error("Original requester for entropy is not trusted (must be the contract itself)")]
    InvalidEntropyRequester {},
}
