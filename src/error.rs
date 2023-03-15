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
    InvalidCoin {},

    #[error("Callback was not called by beacon, but by: {caller}")]
    InvalidEntropyCallback {caller: String},

    #[error("Original requester for entropy is not trusted requester: 
    {requester}, contract: {contract}")]
    InvalidEntropyRequester {requester: String, contract: String},

    #[error("game.result is invalid, should be [0:6]: {msg}")]
    InvalidGameResult { msg: String },

    #[error("Invalid denom: {invalid_denom}, expected: {valid_denom}")]
    InvalidHouseDenom { invalid_denom: String, valid_denom: String },

    #[error("Invalid payout ratio: {got}, expected: {expected}")]
    InvalidPayoutRatio { got: String, expected: String },

    #[error("Invalid denom sent, expected {house_denom}, got: {player_denom}")]
    InvalidDenom { house_denom: String, player_denom: String },

    #[error("Invalid ruleset")]
    InvalidRuleset {},

    #[error("Config does not match expected value {expected}, got: {got}")]
    InvalidConfig { expected: String, got: String },

    #[error("Index does not match expected value {expected}, got: {got}")]
    InvalidIndex { expected: String, got: String },

    #[error("Invalid bet number: {bet}, expected a number between 0 and 6")]
    InvalidBetNumberRange { bet: String },

    #[error("Error: This game has alreay been played")]
    GameAlreadyPlayed {}, 

    #[error("Error: Invalid entropy format or length: {entropy}")]
    InvalidEntropy { entropy: String },
}
