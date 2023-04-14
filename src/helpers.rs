use crate::error::ContractError;
use crate::state::{
    Game, LeaderBoardEntry, PlayerHistory, RuleSet, GAME, IDX, PLAYER_HISTORY, STATE,
};
use cosmwasm_std::{
    from_slice, to_vec, BankMsg, Coin, Deps, DepsMut, Empty, MessageInfo, Response, StdResult,
    Storage, Uint128,
};
use entropy_beacon_cosmos::CalculateFeeQuery;

pub fn calculate_payout(bet_amount: Uint128, outcome: u8) -> Uint128 {
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

pub fn get_outcome_from_entropy(entropy: &Vec<u8>, _rule_set: &RuleSet) -> Vec<u8> {
    let desired_entropy_len: usize = 64;

    if entropy.len() != desired_entropy_len {
        return vec![];
    }

    struct SectorProbabilities {
        zero: u32,
        one: u32,
        two: u32,
        three: u32,
        four: u32,
        five: u32,
        six: u32,
    }

    const ATLANTIC_RULES_SECTOR_FREQUENCY: SectorProbabilities = SectorProbabilities {
        zero: 24,
        one: 12,
        two: 8,
        three: 4,
        four: 2,
        five: 1,
        six: 1,
    };

    // Calculate the total weight (sum of frequencies)
    // by summing the weight of all possible outcomes
    let total_weight = ATLANTIC_RULES_SECTOR_FREQUENCY.zero
        + ATLANTIC_RULES_SECTOR_FREQUENCY.one
        + ATLANTIC_RULES_SECTOR_FREQUENCY.two
        + ATLANTIC_RULES_SECTOR_FREQUENCY.three
        + ATLANTIC_RULES_SECTOR_FREQUENCY.four
        + ATLANTIC_RULES_SECTOR_FREQUENCY.five
        + ATLANTIC_RULES_SECTOR_FREQUENCY.six;

    // Convert the entropy into a number between 0 and total_weight - 1
    // This is done by first extracting the first 4 bytes of the entropy vector
    // then converting them to a u32 integer, then calculating the remainer when dividing by the total weight
    let entropy_number_raw = u32::from_be_bytes(entropy[..4].try_into().unwrap());
    let entropy_number = Uint128::from(entropy_number_raw % total_weight);

    // Determine the outcome based on the weighted random approach
    // by checking which range the entropy number falls into based on the cumulative weights of the outcome
    // similar to spinning a wheel with different sized sectors representing the weight of each outcome

    let outcome: u8;
    let mut weight_sum = ATLANTIC_RULES_SECTOR_FREQUENCY.zero;

    if entropy_number < Uint128::from(weight_sum) {
        outcome = 0;
    } else {
        weight_sum += ATLANTIC_RULES_SECTOR_FREQUENCY.one;
        if entropy_number < Uint128::from(weight_sum) {
            outcome = 1;
        } else {
            weight_sum += ATLANTIC_RULES_SECTOR_FREQUENCY.two;
            if entropy_number < Uint128::from(weight_sum) {
                outcome = 2;
            } else {
                weight_sum += ATLANTIC_RULES_SECTOR_FREQUENCY.three;
                if entropy_number < Uint128::from(weight_sum) {
                    outcome = 3;
                } else {
                    weight_sum += ATLANTIC_RULES_SECTOR_FREQUENCY.four;
                    if entropy_number < Uint128::from(weight_sum) {
                        outcome = 4;
                    } else {
                        weight_sum += ATLANTIC_RULES_SECTOR_FREQUENCY.five;
                        if entropy_number < Uint128::from(weight_sum) {
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
    player_history.total_coins_spent.amount += bet_size;
    player_history.total_coins_won.amount += calculated_payout;

    // Give player a free spin for every 5 games played
    if player_history.games_played == Uint128::new(5) {
        player_history.free_spins += Uint128::new(1);
    }

    player_history
}

pub fn update_player_history_loss(
    player_history: &mut PlayerHistory,
    bet_size: Uint128,
) -> &mut PlayerHistory {
    player_history.games_played += Uint128::new(1);
    player_history.losses += Uint128::new(1);
    player_history.total_coins_spent.amount += bet_size;

    // Give player a free spin for every 5 games played
    if player_history.games_played == Uint128::new(5) {
        player_history.free_spins += Uint128::new(1);
    }

    player_history
}

pub fn update_game_state_for_win(mut game: Game, outcome: &[u8], payout_amount: Uint128) -> Game {
    game.win = true;
    game.played = true;
    game.outcome = outcome[0].to_string();
    game.payout.amount = payout_amount;
    game
}

pub fn update_game_state_for_loss(mut game: Game, outcome: &[u8]) -> Game {
    game.win = false;
    game.played = true;
    game.outcome = outcome[0].to_string();
    game.payout.amount = Uint128::zero();
    game
}

pub fn update_leaderboard(storage: &mut dyn Storage, player: &String, wins: Uint128) {
    let leaderboard_key = "leaderboard";
    let mut leaderboard: Vec<LeaderBoardEntry> = storage
        .get(leaderboard_key.as_bytes())
        .map(|value| from_slice(&value).unwrap())
        .unwrap_or_else(std::vec::Vec::new);

    let entry = leaderboard.iter_mut().find(|entry| &entry.player == player);

    if let Some(existing_entry) = entry {
        existing_entry.wins += wins;
    } else {
        leaderboard.push(LeaderBoardEntry {
            player: player.to_string(),
            wins,
        });
    }

    leaderboard.sort_by(|a, b| b.wins.partial_cmp(&a.wins).unwrap());
    leaderboard.truncate(5);

    storage.set(leaderboard_key.as_bytes(), &to_vec(&leaderboard).unwrap());
}

pub fn query_leaderboard(deps: Deps) -> Vec<LeaderBoardEntry> {
    let leaderboard_key = "leaderboard";
    let leaderboard: Vec<LeaderBoardEntry> = deps
        .storage
        .get(leaderboard_key.as_bytes())
        .map(|value| from_slice(&value).unwrap())
        .unwrap_or_else(std::vec::Vec::new);

    leaderboard
}

pub fn validate_bet_number(bet_number: Uint128) -> Result<(), ContractError> {
    if bet_number > Uint128::new(6) {
        return Err(ContractError::InvalidBetNumber {});
    }
    Ok(())
}

pub fn validate_funds_sent(coin: &Coin, info: &MessageInfo) -> Result<(), ContractError> {
    if coin.amount != info.funds[0].amount {
        return Err(ContractError::ValidateBetFundsSentMismatch {
            player_sent_amount: coin.amount,
            bet_amount: info.funds[0].amount,
        });
    }
    Ok(())
}

pub fn validate_denom(coin: &Coin, bankroll_balance: &Coin) -> Result<(), ContractError> {
    if coin.denom != bankroll_balance.denom {
        return Err(ContractError::ValidateBetDenomMismatch {
            player_sent_denom: coin.denom.clone(),
            house_bankroll_denom: bankroll_balance.denom.clone(),
        });
    }
    Ok(())
}

pub fn validate_bet_amount(coin: &Coin) -> Result<(), ContractError> {
    if coin.amount <= Uint128::new(0) || coin.amount.is_zero() {
        return Err(ContractError::ValidateBetBetAmountIsZero {});
    }
    Ok(())
}

pub fn validate_bet_vs_bankroll(
    info: &MessageInfo,
    bankroll_balance: &Coin,
) -> Result<(), ContractError> {
    if info.funds[0].amount
        > bankroll_balance
            .amount
            .checked_div(Uint128::new(100))
            .unwrap()
    {
        return Err(
            ContractError::ValidateBetBetAmountExceedsHouseBankrollBalance {
                player_bet_amount: info.funds[0].amount,
                house_bankroll_balance: bankroll_balance.amount,
            },
        );
    }
    Ok(())
}

pub fn validate_sent_amount_to_cover_fee(
    sent_amount: Uint128,
    beacon_fee: u64,
) -> Result<(), ContractError> {
    if sent_amount < Uint128::from(beacon_fee) {
        Err(ContractError::InsufficientFunds {})
    } else {
        Ok(())
    }
}

pub fn calculate_beacon_fee(
    deps: &DepsMut,
    sender: &str,
    callback_gas_limit: u64,
) -> Result<u64, ContractError> {
    let state = STATE.load(deps.storage)?;
    let beacon_addr = state.entropy_beacon_addr;

    if sender == "player" {
        Ok(10u64)
    } else {
        CalculateFeeQuery::query(deps.as_ref(), callback_gas_limit, beacon_addr.clone()).map_err(
            |_| ContractError::BeaconFeeError {
                beacon_addr: beacon_addr.to_string(),
                callback_gas_limit,
            },
        )
    }
}

pub fn get_rule_set() -> RuleSet {
    RuleSet {
        zero: Uint128::new(1),
        one: Uint128::new(3),
        two: Uint128::new(5),
        three: Uint128::new(10),
        four: Uint128::new(20),
        five: Uint128::new(45),
        six: Uint128::new(45),
    }
}

pub fn get_bankroll_balance(
    deps: &DepsMut,
    contract_address: &str,
    denom: String,
) -> Result<Coin, ContractError> {
    deps.querier
        .query_balance(contract_address.to_string(), denom)
        .map_err(|_| ContractError::ValidateBetUnableToGetBankrollBalance {
            addr: contract_address.to_string(),
        })
}

pub fn load_player_history_or_create_new(
    storage: &mut dyn Storage,
    sender: String,
) -> Result<PlayerHistory, ContractError> {
    match PLAYER_HISTORY.may_load(storage, sender.clone()) {
        Ok(Some(player_history)) => Ok(player_history),
        Ok(None) => Ok(PlayerHistory::new(sender)),
        Err(_) => Err(ContractError::UnableToLoadPlayerHistory {
            player_addr: sender,
        }),
    }
}

pub fn update_player_history_and_save(
    storage: &mut dyn Storage,
    sender: String,
    player_history: &mut PlayerHistory,
) -> StdResult<()> {
    player_history.free_spins -= Uint128::new(1);
    PLAYER_HISTORY.save(storage, sender, player_history)
}

pub fn save_game_state(storage: &mut dyn Storage, idx: u128, game: &Game) -> StdResult<()> {
    GAME.save(storage, idx, game)
}

pub fn verify_callback_sender(
    sender: &String,
    beacon_addr: &String,
    requester: &String,
    trusted_address: &String,
) -> Result<(), ContractError> {
    if sender == "player" {
        // de nada
    } else if sender != beacon_addr {
        return Err(ContractError::CallBackCallerError {
            caller: sender.to_string(),
            expected: beacon_addr.to_string(),
        });
    }

    if requester == "player" {
        // de nada
    } else if requester != trusted_address {
        return Err(ContractError::EntropyRequestError {
            requester: requester.to_string(),
            trusted: trusted_address.to_string(),
        });
    }

    Ok(())
}

pub fn update_game_and_player_history(
    win: bool,
    game: &Game,
    player_history: &mut PlayerHistory,
    outcome: &[u8],
) -> (Game, Uint128) {
    if win {
        let calculated_payout = calculate_payout(game.bet_size.into(), outcome[0]);

        let updated_game = update_game_state_for_win(game.clone(), outcome, calculated_payout);
        update_player_history_win(
            player_history,
            Uint128::from(game.bet_size),
            calculated_payout,
        );
        (updated_game, calculated_payout)
    } else {
        let updated_game = update_game_state_for_loss(game.clone(), outcome);
        update_player_history_loss(player_history, Uint128::from(game.bet_size));
        (updated_game, Uint128::new(0))
    }
}

pub fn build_response(win: bool, game: &Game, payout: Uint128) -> Response<Empty> {
    if win {
        Response::new()
            .add_message(BankMsg::Send {
                to_address: game.player.to_string(),
                amount: vec![Coin {
                    denom: "ukuji".to_string(),
                    amount: payout,
                }],
            })
            .add_attribute("game_result", win.to_string())
            .add_attribute("game_outcome", game.outcome.clone())
            .add_attribute("game_payout", payout.to_string())
    } else {
        Response::new()
            .add_attribute("game_result", win.to_string())
            .add_attribute("game_outcome", game.outcome.clone())
            .add_attribute("game_payout", Uint128::new(0).to_string())
    }
}

pub fn process_game_result(
    storage: &mut dyn Storage,
    outcome: &[u8],
    game: &Game,
    player_history: &mut PlayerHistory,
    idx: &mut Uint128,
) -> Result<Response<Empty>, ContractError> {
    let win = game.is_winner(game.bet_number.into(), outcome.to_owned());

    let (updated_game, payout) = update_game_and_player_history(win, game, player_history, outcome);

    GAME.save(storage, (*idx).into(), &updated_game)?;
    PLAYER_HISTORY.save(storage, game.player.to_string(), player_history)?;

    *idx += Uint128::new(1);
    IDX.save(storage, idx)?;

    // Update the leaderboard if the player has won
    if win {
        update_leaderboard(storage, &game.player, Uint128::new(1));
    }

    let response = build_response(win, &updated_game, payout);

    Ok(response)
}
