use cosmwasm_schema::cw_serde;
use kujira::denom::Denom;

use cosmwasm_std::{Addr, Uint128, Coin};
use cw_storage_plus::{Item, Map};

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct State {
    pub entropy_beacon_addr: Addr,
    pub owner_addr: Addr,
    pub token: Denom,
    pub house_bankroll: Coin, 
    pub play_amount: Uint128,
    pub win_amount: Uint128,
    pub fee_amount: Uint128,
    pub rule_set: RuleSet,
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct Game {
    pub player: Addr,
    pub bet: u8,
    pub payout: Uint128,
    pub result: Option<Vec<u8>>,
    pub played: bool,
    pub win: Option<bool>, 
}

impl Game {
    // Cheks if player bet matches the outcome generated by the entropy beacon
    pub fn win(&self, player_bet: u8) -> bool {
        match &self.result {
            Some(result) if result.contains(&player_bet) => true,
            _ => false,
        }
    }
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct RuleSet {
    pub zero: Uint128,
    pub one: Uint128,
    pub two: Uint128,
    pub three: Uint128,
    pub four: Uint128,
    pub five: Uint128,
    pub six: Uint128,
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct EntropyCallbackData {
    pub original_sender: Addr,
}

pub const IDX: Item<Uint128> = Item::new("idx");
pub const GAME: Map<u128, Game> = Map::new("game");
pub const STATE: Item<State> = Item::new("state");
