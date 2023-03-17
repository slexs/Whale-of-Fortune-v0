use cosmwasm_std::{DepsMut, Env, MessageInfo, Uint128};
use cw_utils::one_coin;
use sha2::{Digest, Sha256};

use crate::state::{RuleSet};

pub fn calculate_payout(bet_amount: Uint128, outcome: u8, rule_set: RuleSet) -> Uint128 {
    match outcome {
        0 => bet_amount.checked_mul(rule_set.zero).unwrap_or(Uint128::zero()),
        2 => bet_amount.checked_mul(rule_set.two).unwrap_or(Uint128::zero()),
        1 => bet_amount.checked_mul(rule_set.one).unwrap_or(Uint128::zero()),
        3 => bet_amount.checked_mul(rule_set.three).unwrap_or(Uint128::zero()),
        4 => bet_amount.checked_mul(rule_set.four).unwrap_or(Uint128::zero()),
        5 => bet_amount.checked_mul(rule_set.five).unwrap_or(Uint128::zero()),
        6 => bet_amount.checked_mul(rule_set.six).unwrap_or(Uint128::zero()),
        _ => Uint128::zero(),
    }
}

// Take the entropy and return a random number between 0 and 6
pub fn get_outcome_from_entropy(entropy: &Vec<u8>, rule_set: &RuleSet) -> Option<Vec<u8>> {
    // Calculate the total weight (sum of frequencies)
    let total_weight = rule_set.zero
        + rule_set.one
        + rule_set.two
        + rule_set.three
        + rule_set.four
        + rule_set.five
        + rule_set.six;

   // Convert the entropy into a number between 0 and total_weight - 1
   let entropy_number_raw = u32::from_be_bytes(entropy[..4].try_into().unwrap());
   let entropy_number = Uint128::from(entropy_number_raw) % total_weight;


    // Determine the outcome based on the weighted random approach
    let mut outcome = 0;
    let mut weight_sum = rule_set.zero;

    if entropy_number < weight_sum {
        outcome = 0;
    } else {
        weight_sum += rule_set.one;
        if entropy_number < weight_sum {
            outcome = 1;
        } else {
            weight_sum += rule_set.two;
            if entropy_number < weight_sum {
                outcome = 2;
            } else {
                weight_sum += rule_set.three;
                if entropy_number < weight_sum {
                    outcome = 3;
                } else {
                    weight_sum += rule_set.four;
                    if entropy_number < weight_sum {
                        outcome = 4;
                    } else {
                        weight_sum += rule_set.five;
                        if entropy_number < weight_sum {
                            outcome = 5;
                        } else {
                            outcome = 6;
                        }
                    }
                }
            }
        }
    }

    // Return the outcome as a single-element vector
    Some(vec![outcome])
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

    // Check that the players bet amount is not zero 
    if coin.amount <= Uint128::new(0) || coin.amount.is_zero(){
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
