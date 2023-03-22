use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Uint128};
use cw_utils::one_coin;
use sha2::{Digest, Sha256};

use crate::error::ContractError;

use crate::state::RuleSet;

pub fn calculate_payout(bet_amount: Uint128, outcome: u8, rule_set: RuleSet) -> Uint128 {
    match outcome {
        0 => bet_amount
            .checked_mul(Uint128::new(1))
            .unwrap_or(Uint128::zero()),
        1 => bet_amount
            .checked_mul(Uint128::new(3))
            .unwrap_or(Uint128::zero()),
        2 => bet_amount
            .checked_mul(Uint128::new(5))
            .unwrap_or(Uint128::zero()),
        3 => bet_amount
            .checked_mul(Uint128::new(10))
            .unwrap_or(Uint128::zero()),
        4 => bet_amount
            .checked_mul(Uint128::new(20))
            .unwrap_or(Uint128::zero()),
        5 => bet_amount
            .checked_mul(Uint128::new(45))
            .unwrap_or(Uint128::zero()),
        6 => bet_amount
            .checked_mul(Uint128::new(45))
            .unwrap_or(Uint128::zero()),
        _ => Uint128::zero(),
    }
}

// Distribute outcomes according to the frequency of each sector in the Big Six wheel game
// Take the entropy and return a random number between 0 and 6
// Designed to generate a random outcome
pub fn get_outcome_from_entropy(entropy: &Vec<u8>, rule_set: &RuleSet) -> Vec<u8> {
    // Calculate the total weight (sum of frequencies)
    // by summing the weight of all possible outcomes
    let total_weight = rule_set.zero
        + rule_set.one
        + rule_set.two
        + rule_set.three
        + rule_set.four
        + rule_set.five
        + rule_set.six;

    // Convert the entropy into a number between 0 and total_weight - 1
    // This is done by first extracting the first 4 bytes of the entropy vector
    // then converting them to a u32 integer, then calculating the remainer when dividing by the total weight
    let entropy_number_raw = u32::from_be_bytes(entropy[..4].try_into().unwrap());
    let entropy_number = Uint128::from(entropy_number_raw) % total_weight;

    // Determine the outcome based on the weighted random approach
    // by checking which range the entropy number falls into based on the cumulative weights of the outcome
    // similar to spinning a wheel with different sized sectors representing the weight of each outcome
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
    vec![outcome]
}

pub fn execute_validate_bet(
    deps: &DepsMut,
    env: &Env,
    info: MessageInfo,
    player_bet_amount: Uint128,
    player_bet_number: Uint128,
) -> Result<Response, ContractError> {
    // Get the balance of the house bankroll (contract address balance)
    let bankroll_balance = deps
        .querier
        .query_balance(env.contract.address.to_string(), "ukuji".to_string());

    if bankroll_balance.is_err() {
        return Err(ContractError::ValidateBetUnableToGetBankrollBalance {
            house_bankroll_addr: env.contract.address.to_string(),
        });
    }

    let bankroll_balance = bankroll_balance.unwrap();

    // Check that the players bet number is between 0 and 6
    if player_bet_number > Uint128::new(6) {
        return Err(ContractError::InvalidBetNumber {});
    }

    // Check that only one denom was sent
    let coin = one_coin(&info);
    if coin.is_err() {
        return Err(ContractError::ValidateBetInvalidDenom {});
    }

    let coin = coin.unwrap();

    // Check that the denom is the same as the token in the bankroll ("ukuji")
    if coin.denom != bankroll_balance.denom {
        return Err(ContractError::ValidateBetDenomMismatch {
            player_sent_denom: coin.denom,
            house_bankroll_denom: bankroll_balance.denom,
        });
    }

    // Check that the players bet amount is not zero
    if coin.amount <= Uint128::new(0) || coin.amount.is_zero() {
        return Err(ContractError::ValidateBetBetAmountIsZero {});
    }

    // Ensure that the amount of funds sent by player matches bet size
    if coin.amount != player_bet_amount {
        return Err(ContractError::ValidateBetFundsSentMismatch {
            funds_sent: coin.amount,
            bet_size: player_bet_amount,
        });
    }

    /* Make sure the player's bet_amount does not exceed 1% of house bankroll
    Ex: House Bankroll 1000, player bets 10, max player payout is 450 */
    if player_bet_amount
        > bankroll_balance
            .amount
            .checked_div(Uint128::new(100))
            .unwrap()
    {
        return Err(
            ContractError::ValidateBetBetAmountExceedsHouseBankrollBalance {
                player_bet_amount: player_bet_amount,
                house_bankroll_balance: bankroll_balance.amount,
            },
        );
    }

    Ok(Response::default())
}
