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

    #[error("Unable to fetch bankroll balance from addr: {addr}")]
    ValidateBetUnableToGetBankrollBalance { addr: String },

    #[error("Invalid bet number, bet numbers must be between 0 and 6")]
    InvalidBetNumber {},

    #[error("Error in loading player_history for {player_addr}")]
    UnableToLoadPlayerHistory{ player_addr: String },

    #[error("Invalid bet denom, only one denom is allowed per bet")]
    ValidateBetInvalidDenom {},

    #[error("Bet denom mismatch, player sent denom: {player_sent_denom}, house bankroll denom: {house_bankroll_denom}")]
    ValidateBetDenomMismatch {
        player_sent_denom: String,
        house_bankroll_denom: String,
    }, 

    #[error("Bet amount is zero")]
    ValidateBetBetAmountIsZero{}, 

    #[error("Bet amount mismatch, player sent amount: {player_sent_amount}, bet amount: {bet_amount}")]
    ValidateBetFundsSentMismatch {
        player_sent_amount: Uint128,
        bet_amount: Uint128,
    },


    #[error("Bet amount exceeds 1% of house bankroll balance, bet amount: {player_bet_amount}, house bankroll balance: {house_bankroll_balance}")]
    ValidateBetBetAmountExceedsHouseBankrollBalance {
        player_bet_amount: Uint128,
        house_bankroll_balance: Uint128,
    },

    #[error("UnableToLoadGameIndex")]
    UnableToLoadGameIndex{},

    #[error("No Freespins left")]
    NoFreeSpinsLeft {}, 

    #[error("Calculate beacon fee error, 
    BeaconAddr: {beacon_addr}, 
    callbackGasLimit: {callback_gas_limit}")]
    BeaconFeeError { beacon_addr: String, callback_gas_limit: u64},

}
