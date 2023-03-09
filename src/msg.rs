use crate::state::RuleSet;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Api, Coin, StdResult, Uint128};
use cw20::{Cw20Coin, Cw20ReceiveMsg};
use entropy_beacon_cosmos::EntropyCallbackMsg;
use kujira::denom::Denom;

#[cw_serde]
pub struct InstantiateMsg {
    pub entropy_beacon_addr: Addr,
    pub owner_addr: Addr,
    pub token: Denom,
    pub play_amount: Uint128,
    pub win_amount: Uint128,
    pub fee_amount: Uint128,
    pub rule_set: RuleSet,
}

#[cw_serde]
pub struct EntropyCallbackData {
    pub game: Uint128,
    pub original_sender: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    // ValidateBet {
    //     player_bet_amount: Uint128,
    //     player_bet_number: u8,
    // },
    Spin {
        // player_bet_amount: Uint128,
        // player_bet_number: u8,
    },
    Pull {},
    ReceiveEntropy(EntropyCallbackMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GameResponse)]
    Game { idx: Uint128 },
}

#[cw_serde]
pub struct GameResponse {
    pub idx: Uint128,
    pub player: Addr,
    pub result: Option<Vec<u8>>,
    pub win: bool,
}

impl ExecuteMsg {
    pub fn calculate_payout(bet_amount: Uint128, result: u8, rule_set: RuleSet) -> Uint128 {
        match result {
            0 => bet_amount * rule_set.zero,
            1 => bet_amount * rule_set.one,
            2 => bet_amount * rule_set.two,
            3 => bet_amount * rule_set.three,
            4 => bet_amount * rule_set.four,
            5 => bet_amount * rule_set.five,
            6 => bet_amount * rule_set.six,
            _ => Uint128::zero(),
        }
    }
}

#[cw_serde]
pub struct MigrateMsg {
    pub fee_amount: Uint128,
}
/*
#[cw_serde]
pub struct CreateMsg {
    /// id is a human-readable name for the escrow to use later
    /// 3-20 bytes of utf-8 text
    pub id: String,
    /// arbiter can decide to approve or refund the escrow
    pub arbiter: String,
    /// if approved, funds go to the recipient
    pub recipient: Option<String>,
    /// Title of the escrow
    pub title: String,
    /// Longer description of the escrow, e.g. what conditions should be met
    pub description: String,
    /// When end height set and block height exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_height: Option<u64>,
    /// When end time (in seconds since epoch 00:00:00 UTC on 1 January 1970) is set and
    /// block time exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_time: Option<u64>,
    /// Besides any possible tokens sent with the CreateMsg, this is a list of all cw20 token addresses
    /// that are accepted by the escrow during a top-up. This is required to avoid a DoS attack by topping-up
    /// with an invalid cw20 contract. See https://github.com/CosmWasm/cosmwasm-plus/issues/19
    pub cw20_whitelist: Option<Vec<String>>,
}

impl CreateMsg {
    pub fn addr_whitelist(&self, api: &dyn Api) -> StdResult<Vec<Addr>> {
        match self.cw20_whitelist.as_ref() {
            Some(v) => v.iter().map(|h| api.addr_validate(h)).collect(),
            None => Ok(vec![]),
        }
    }
}

#[cw_serde]
pub struct DetailsResponse {
    /// id of this escrow
    pub id: String,
    /// arbiter can decide to approve or refund the escrow
    pub arbiter: String,
    /// if approved, funds go to the recipient
    pub recipient: Option<String>,
    /// if refunded, funds go to the source
    pub source: String,
    /// Title of the escrow
    pub title: String,
    /// Longer description of the escrow, e.g. what conditions should be met
    pub description: String,
    /// When end height set and block height exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_height: Option<u64>,
    /// When end time (in seconds since epoch 00:00:00 UTC on 1 January 1970) is set and
    /// block time exceeds this value, the escrow is expired.
    /// Once an escrow is expired, it can be returned to the original funder (via "refund").
    pub end_time: Option<u64>,
    /// Balance in native tokens
    pub native_balance: Vec<Coin>,
    /// Balance in cw20 tokens
    pub cw20_balance: Vec<Cw20Coin>,
    /// Whitelisted cw20 tokens
    pub cw20_whitelist: Vec<String>,
}

#[cw_serde]
pub enum ReceiveMsg {
    Create(CreateMsg),
    /// Adds all sent native tokens to the contract
    TopUp {
        id: String,
    },
}

#[cw_serde]
pub enum ExecuteMsgEscrow {
    Create(CreateMsg),
    /// Adds all sent native tokens to the contract
    TopUp {
        id: String,
    },
    /// Set the recipient of the given escrow
    SetRecipient {
        id: String,
        recipient: String,
    },
    /// Approve sends all tokens to the recipient.
    /// Only the arbiter can do this
    Approve {
        /// id is a human-readable name for the escrow from create
        id: String,
    },
    /// Refund returns all remaining tokens to the original sender,
    /// The arbiter can do this any time, or anyone can do this after a timeout
    Refund {
        /// id is a human-readable name for the escrow from create
        id: String,
    },
    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
} */
