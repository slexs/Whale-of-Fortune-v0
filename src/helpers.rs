use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, Uint128, Response};

use cw_utils::one_coin;
use sha2::{Digest, Sha256};

use crate::{state::{Game, PlayerHistory, RuleSet, STATE, IDX, PLAYER_HISTORY}, ContractError};

// pub fn handle_spin(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     bet_number: Uint128,
//     bet_size: Uint128,
//     is_free_spin: bool,
// ) -> Result<Response, ContractError> {
//     let state = STATE.load(deps.storage)?;
//     let beacon_addr = state.entropy_beacon_addr;

//     let idx = IDX.load(deps.storage)?;

//     // Create a new game state
//     let game = Game::new_game(&info.sender.to_string(), idx.into(), bet_number.into(), bet_size.into());

//     // Get the balance of the house bankroll (contract address balance)
//     let bankroll_balance = deps
//         .querier
//         .query_balance(env.contract.address.to_string(), "ukuji".to_string());

//     // Gracefully handle any errors 
//     let bankroll_balance = bankroll_balance.map_err(|_| ContractError::ValidateBetUnableToGetBankrollBalance {
//         addr: env.contract.address.to_string(),
//     })?;

//     // Check that the players bet number is between 0 and 6
//     if bet_number > Uint128::new(6) {
//         return Err(ContractError::InvalidBetNumber {});
//     }

//     // Ensure that the amount of funds sent by player matches bet size if it's not a free spin
//     if !is_free_spin {
//         let coin = match one_coin(&info) {
//             Ok(coin) => coin,
//             Err(_) => return Err(ContractError::ValidateBetInvalidDenom {}),
//         };

//     // Check that the sent amount matches the bet size
//     if coin.amount != bet_size {
//         return Err(ContractError::InvalidBetAmount {});
//     }

//     // Calculate the max winnings based on the bet size
//     let max_win = calculate_max_win(bet_size);

//     // Ensure the house has enough funds to cover max winnings
//     if bankroll_balance.amount < max_win {
//         return Err(ContractError::InsufficientBankrollFunds {});
//     }

//     // Load player history
//     let mut player_history = PLAYER_HISTORY.load(deps.storage, info.sender.clone())?;

//     // If it's not a free spin, update the player's total bet amount
//     if !is_free_spin {
//         player_history.total_bet_amount += bet_size;
//     }

//     // Deduct one credit from player_history.free_spins if it's a free spin
//     if is_free_spin {
//         if player_history.free_spins == Uint128::zero() {
//             return Err(ContractError::NoFreeSpinsLeft {});
//         }
//         player_history.free_spins -= Uint128::new(1);
//     }

//     // Save the updated player history
//     PLAYER_HISTORY.save(deps.storage, info.sender.clone(), &player_history)?;

//     // Save the game state
//     GAMES.save(deps.storage, idx, &game)?;

//     // Increment the game index
//     IDX.save(deps.storage, &(idx + 1))?;

//     // Request entropy from the entropy beacon
//     let entropy_request = WasmMsg::Execute {
//         contract_addr: beacon_addr,
//         msg: to_binary(&EntropyRequestMsg {
//             callback_addr: env.contract.address.to_string(),
//         })?,
//         funds: vec![],
//     };
//     let res = Response::new()
//         .add_attribute("action", "spin")
//         .add_message(entropy_request);

//     Ok(res)
// }


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

    let expected_entropy_len: usize = 64; 

    // Check if entropy is empty, if so return an empty vector
    if entropy.is_empty() {
        return vec![];
    } else if entropy.len() != expected_entropy_len {
        return vec![];
    }

    // Check if entropy is shoter than 

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
    let mut weight_sum = total_weight;

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

pub fn update_player_history_win(
    player_history: &mut PlayerHistory,
    bet_size: Uint128,
    calculated_payout: Uint128,
) -> &mut PlayerHistory {
    player_history.games_played += Uint128::new(1);
    player_history.wins += Uint128::new(1);
    player_history.total_coins_spent = Coin {
        amount: bet_size,
        denom: "ukuji".to_string(),
    };
    player_history.total_coins_won = Coin {
        amount: calculated_payout,
        denom: "ukuji".to_string(),
    };
    player_history
}

pub fn update_player_history_loss(
    player_history: &mut PlayerHistory,
    bet_size: Uint128,
) -> &mut PlayerHistory {
    player_history.games_played += Uint128::new(1);
    player_history.losses += Uint128::new(1);
    player_history.total_coins_spent = Coin {
        amount: bet_size,
        denom: "ukuji".to_string(),
    };
    player_history
}

pub fn update_game_state_for_win(
    mut game: Game,
    outcome: &Vec<u8>,
    payout_amount: Uint128,
) -> Game {
    game.win = true;
    game.played = true;
    game.outcome = outcome[0].to_string();
    game.payout.amount = payout_amount;
    game
}

pub fn update_game_state_for_loss(mut game: Game, outcome: &Vec<u8>) -> Game {
    game.win = false;
    game.played = true;
    game.outcome = outcome[0].to_string();
    game.payout.amount = Uint128::zero();
    game
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
    if coin.amount <= Uint128::new(0) || coin.amount.is_zero() {
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
