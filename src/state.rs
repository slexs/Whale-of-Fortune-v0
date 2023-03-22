use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::Item;
use cw_storage_plus::Map;

// State struct represents the state of the contract
#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct State {
    pub entropy_beacon_addr: Addr,
    pub house_bankroll: Coin,
}

// A struct to represent a game
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

// Implement the is_winner method for the Game struct
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
    }
}

// A struct to represent the ruleset used in the game state
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

// A struct to represent the player history, used for leaderboard and loyalty points
#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct PlayerHistory {
    pub player: String,
    pub games_played: Uint128,
    pub games_won: Uint128,
    pub games_lost: Uint128,
    pub total_winnings: Uint128,
    pub total_losses: Uint128,
    pub loyalty_points: Uint128,
}

pub const IDX: Item<Uint128> = Item::new("idx");
pub const GAME: Map<u128, Game> = Map::new("game");
pub const STATE: Item<State> = Item::new("state");
pub const PLAYER_HISTORY: Map<String, PlayerHistory> = Map::new("player_history");
