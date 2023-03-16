use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Invalid bet")]
    InvalidBet {},

    #[error("callback not called by beacon")]
    CallBackCallerError {},

    #[error("Requester for entropy is not trusted")]
    EntropyRequestError {},
}
