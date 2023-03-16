use cosmwasm_schema::cw_serde;

use std::fmt; 
use kujira::denom::Denom;

use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub entropy_beacon_addr: Addr,
    pub owner_addr: Addr,
    pub house_bankroll: Coin, 
    pub fee_amount: Uint128,
    pub rule_set: RuleSet,
    pub token: Denom,
}
impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Config {{ entropy_beacon_addr: {}, owner_addr: {}, house_bankroll: {}, token: {}, fee_amount: {}, rule_set: {:?} }}",
            self.entropy_beacon_addr, self.owner_addr, self.house_bankroll, self.token, self.fee_amount, self.rule_set
        )
    }
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct Game {
    pub player: Addr,
    pub bet_number: Uint128,
    pub bet_size: Uint128, 
    pub payout: Uint128,
    pub result: Option<Vec<u8>>,
    pub played: bool,
    pub win: Option<bool>,
    pub game_id: Uint128,
    pub entropy_requested: bool, 
}

impl Game {
    // Cheks if player bet matches the outcome generated by the entropy beacon
    pub fn win(&self, player_bet: Uint128) -> bool {
            // Convert the result vector to a vector of u128 values
            let result: Vec<u128> = self.result.clone().unwrap_or_default().into_iter().map(|x| u128::from(x)).collect();

            // Convert the player_bet Uint128 to a u128 value
            let player_bet_u128 = player_bet.u128();

            // Check if the player bet matches the outcome generated by the entropy beacon
            result.contains(&player_bet_u128)
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
// pub const GAME: Map<u128, Game> = Map::new("game");
pub const GAME: Item<Game> = Map::new("game");
pub const CONFIG: Item<Config> = Item::new("state");
