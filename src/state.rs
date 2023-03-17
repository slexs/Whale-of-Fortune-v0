use cosmwasm_schema::cw_serde;
use cw_storage_plus::Map;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::Item;

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct State {
    pub entropy_beacon_addr: Addr,
    pub house_bankroll: Coin,
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct Game {
    pub player: String,
    pub game_idx: u128,
    pub bet_number: u128,
    pub bet_size: u128,
    pub outcome: String,
    pub played: bool,
    pub win: bool,
    pub payout: Coin, 
    pub rule_set: RuleSet,
}

impl Game {
    pub fn is_winner(&self, player_bet: Uint128, outcome: Vec<u8>) -> bool {

        // Check that outcome is not empty 
        if outcome.is_empty() {
            return false; 
        }

        // Get the first byte of the outcome as a u128 value
        let outcome_value = u128::from(outcome[0]);

        // Compare the player_bet Uint128 to the outcome_value 
        player_bet.u128() == outcome_value

        // // Convert the outcome vector into a vector of u128 values
        // let chunk_size = std::mem::size_of::<u128>();
        // let result: Vec<u128> = outcome
        //     .chunks(chunk_size)
        //     .filter(|chunk| chunk.len() == chunk_size)
        //     .map(|chunk| u128::from_be_bytes(chunk.try_into().unwrap()))
        //     .collect();

        // // Convert the player_bet Uint128 to a u128 value
        // let player_bet_u128 = player_bet.u128();

        // // Check if the player bet matches the outcome generated by the entropy beacon
        // result.contains(&player_bet_u128)
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

pub const IDX: Item<Uint128> = Item::new("idx");
pub const GAME: Map<u128, Game> = Map::new("game");
pub const STATE: Item<State> = Item::new("state");
