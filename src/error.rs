use cosmwasm_std::{Uint128};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("StdError: {0}")]
    StdError(#[from] cosmwasm_std::StdError),

    #[error("CustomStdError: {0}")]
    QueryError(String),

    #[error("Game not found at index {0}")]
    GameNotFound(Uint128), 

    #[error("Invalid execute message, got {got}, expected: 'spin' with a bet number between 0 and 6")]
    InvalidExecuteMsg { got: String },

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Invalid bet")]
    InvalidBet {},

    #[error("callback not called by beacon, caller: {caller}, expected: {expected}")]
    CallBackCallerError {caller: String, expected: String},

    #[error("Requester for entropy is not trusted, requester: {requester}, trusted: {trusted}")]
    EntropyRequestError {requester: String, trusted: String},

    #[error("Entropy result is invalid, result: {result}")]
    InvalidEntropyResult { result: String },

    #[error("Player {player} has no history, cannot redeem loyalty points")]
    NoPlayerHistory {player: String}, 

    #[error("Player {player} has no history")]
    PlayerHistoryLoadError {player: String}, 
}
