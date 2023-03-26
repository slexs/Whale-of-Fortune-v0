use std::fmt;

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
    }
}

#[cw_serde]
#[serde(rename_all = "snake_case")]
pub struct PlayerHistory {
    pub player_address: String,
    pub games_played: Uint128,
    pub wins: Uint128,
    pub losses: Uint128, 
    pub win_loss_ratio: Uint128,
    pub total_coins_spent: Coin, 
    pub total_coins_won: Coin,
}

impl fmt::Display for PlayerHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "player_address: {}, games_played: {}, wins: {}, losses: {}, win_loss_ratio: {}, total_coins_spent: ({} {}), total_coins_won: ({} {})",
            self.player_address,
            self.games_played,
            self.wins,
            self.losses,
            self.win_loss_ratio,
            self.total_coins_spent.amount,
            self.total_coins_spent.denom,
            self.total_coins_won.amount,
            self.total_coins_won.denom,
        )
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
pub const PLAYER_HISTORY: Map<String, PlayerHistory> = Map::new("player_history");
