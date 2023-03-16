use cosmwasm_std::{DepsMut, Env, MessageInfo, Uint128};
use cw_utils::one_coin;
use sha2::{Digest, Sha256};

use crate::state::{RuleSet};

pub fn calculate_payout(bet_amount: Uint128, outcome: u8, rule_set: RuleSet) -> Uint128 {
    match outcome {
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

// Take the entropy and return a random number between 0 and 6
pub fn get_outcome_from_entropy(entropy: &[u8]) -> Vec<u8> {
    // Hash the input entropy using SHA256
    let mut hasher = Sha256::new();
    hasher.update(entropy);
    let hash_result = hasher.finalize();

    // Use the last byte of the hash as the random number
    let random_byte = hash_result[hash_result.len() - 1];

    // Map the random byte to a number between 0 and 6
    let outcome = random_byte % 7;
    vec![outcome]
}

pub fn execute_validate_bet(
    deps: &DepsMut,
    env: &Env,
    info: MessageInfo,
    player_bet_amount: Uint128,
    player_bet_number: Uint128,
) -> bool {
    // Get the balance of the house bankroll (contract address balance)
    let bankroll_balance = match deps
        .querier
        .query_balance(env.contract.address.to_string(), "ukuji".to_string())
    {
        Ok(balance) => balance,
        Err(_) => return false,
    };

    // Check that the players bet number is between 0 and 6
    if player_bet_number > Uint128::new(6) {
        return false;
    }

    // Check that only one denom was sent
    let coin = match one_coin(&info) {
        Ok(coin) => coin,
        Err(_) => return false,
    };

    // Check that the denom is the same as the token in the bankroll ("ukuji")
    if coin.denom != bankroll_balance.denom {
        return false;
    }

    // Ensure that the amount of funds sent by player matches bet size
    if coin.amount != player_bet_amount {
        return false;
    }

    /* Make sure the player's bet_amount does not exceed 1% of house bankroll
    Ex: House Bankroll 1000, player bets 10, max player payout is 450 */
    if player_bet_amount
        > bankroll_balance
            .amount
            .checked_div(Uint128::new(100))
            .unwrap()
    {
        return false;
    }

    true
}
