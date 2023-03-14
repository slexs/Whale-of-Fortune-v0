use crate::state::RuleSet;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, /*Api, Coin, StdResult,*/ Uint128};
// use cw20::{Cw20Coin, Cw20ReceiveMsg};
use entropy_beacon_cosmos::EntropyCallbackMsg;
// use kujira::denom::Denom;
// use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct InstantiateMsg {
    // pub entropy_beacon_addr: Addr,
    // pub owner_addr: Addr,
    // pub token: Denom,
    // pub play_amount: Uint128,
    // pub win_amount: Uint128,
    // pub fee_amount: Uint128,
    // pub rule_set: RuleSet,
}

#[cw_serde]
pub struct EntropyCallbackData {
    pub game: Uint128,
    pub original_sender: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    Pull {
        // player_bet_amount: Uint128,
        bet_number: Uint128,
    },

    ReceiveEntropy(EntropyCallbackMsg),
    //TODO: Move Spin() inside RecieveEntropy()
    // Spin {
    //     // player_bet_amount: Uint128,
    //     bet_number: Uint128,
    // },
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
    pub player_bet_number: Uint128,
    pub result: Option<Vec<u8>>,
    pub win: bool,
    pub entropy_requested: bool, 
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
