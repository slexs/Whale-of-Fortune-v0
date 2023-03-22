use cosmwasm_std::Uint128;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("StdError: {0}")]
    StdError(#[from] cosmwasm_std::StdError),

    #[error("CustomStdError: {0}")]
    QueryError(String),

    #[error("Game not found at index {0}")]
    GameNotFound(Uint128),

    #[error(
        "Invalid execute message, got {got}, expected: 'spin' with a bet number between 0 and 6"
    )]
    InvalidExecuteMsg { got: String },

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Invalid bet")]
    InvalidBet {},

    #[error("Invalid betnumber [0+,6]")]
    InvalidBetNumber {},

    #[error("callback not called by beacon, caller: {caller}, expected: {expected}")]
    CallBackCallerError { caller: String, expected: String },

    #[error("Requester for entropy is not trusted, requester: {requester}, trusted: {trusted}")]
    EntropyRequestError { requester: String, trusted: String },

    #[error("Entropy result is invalid, result: {result}")]
    InvalidEntropyResult { result: String },

    #[error("Player {player} has no history, cannot redeem loyalty points")]
    NoPlayerHistory { player: String },

    #[error("Player {player} has no history")]
    PlayerHistoryLoadError { player: String },

    #[error("Unable to get balance of house bankroll: {house_bankroll_addr}")]
    ValidateBetUnableToGetBankrollBalance { house_bankroll_addr: String },

    #[error("Invalid bet, player sent more than one denom")]
    ValidateBetInvalidDenom {},

    #[error(
        "Denom does not match house bankroll denom: {player_sent_denom} != {house_bankroll_denom}"
    )]
    ValidateBetDenomMismatch {
        player_sent_denom: String,
        house_bankroll_denom: String,
    },

    #[error("Funds sent: {funds_sent} does not match bet-size {bet_size}")]
    ValidateBetFundsSentMismatch {
        funds_sent: Uint128,
        bet_size: Uint128,
    },

    #[error("Players bet amount is zero")]
    ValidateBetBetAmountIsZero {},

    #[error("Players bet amount {player_bet_amount} exceeds 1% of house bankroll balance {house_bankroll_balance}")]
    ValidateBetBetAmountExceedsHouseBankrollBalance {
        player_bet_amount: Uint128,
        house_bankroll_balance: Uint128,
    },
}
