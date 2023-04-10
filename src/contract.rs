#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, Uint128, Addr};
use cw2::set_contract_version;

use cw_utils::one_coin;
use entropy_beacon_cosmos::beacon::CalculateFeeQuery;
use entropy_beacon_cosmos::EntropyRequest;

use crate::error::ContractError;
use crate::msg::{
    EntropyCallbackData, ExecuteMsg, GameResponse, InstantiateMsg, MigrateMsg, QueryMsg,
};
use crate::state::{Game, RuleSet, PlayerHistory, State, GAME, IDX, STATE, PLAYER_HISTORY, LatestGameIndexResponse};

use crate::helpers::{calculate_payout, update_leaderboard, query_leaderboard, get_outcome_from_entropy, update_player_history_win, update_game_state_for_win, update_game_state_for_loss, update_player_history_loss, validate_bet_number, validate_denom, validate_bet_amount, validate_funds_sent, validate_bet_vs_bankroll, calculate_beacon_fee, validate_sent_amount_to_cover_fee, get_rule_set, get_bankroll_balance, load_player_history_or_create_new, update_player_history_and_save, verify_callback_sender, process_game_result};

// use cw_storage_plus::Map;

// version info for migration info
const CONTRACT_NAME: &str = "Whale-of-fortune-v1.2.2";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Our [`InstantiateMsg`] contains the address of the entropy beacon contract.
/// We save this address in the contract state.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        entropy_beacon_addr: msg.entropy_beacon_addr.clone(),
        house_bankroll: Coin {
            // Initialize house bankroll to 0
            denom: "ukuji".to_string(),
            amount: Uint128::zero(),
        },
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    let idx = Uint128::zero();

    IDX.save(deps.storage, &idx)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("entropy_beacon_addr", msg.entropy_beacon_addr.to_string())
        .add_attribute("state", format!("{:?}", state)))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Spin { bet_number } => {
            let state = STATE.load(deps.storage)?;
            let idx = IDX.load(deps.storage)?;
            let beacon_addr = state.entropy_beacon_addr;

            let sent_amount: Uint128 = info.funds.iter().map(|c| c.amount).sum();

            // How much gas our callback will use. This is an educated guess, so we usually want to overestimate.
            let callback_gas_limit = 150_000u64;

            // The beacon allows us to query the fee it will charge for a request, given the gas limit we provide.
            let beacon_fee = calculate_beacon_fee(&deps, &info.sender.to_string(), callback_gas_limit)?; 

            validate_sent_amount_to_cover_fee(sent_amount, beacon_fee)?;

            let rule_set =  get_rule_set(); 

            // Create a new game state
            let mut game = Game::new_game(
                &info.sender.to_string(), 
                idx.into(), 
                bet_number.into(), 
                sent_amount.into()
            );

            game.rule_set = rule_set;

            // Get the balance of the house bankroll (contract address balance)
            let bankroll_balance = get_bankroll_balance(&deps, &env.contract.address.to_string(), "ukuji".to_string())?;

            // Check that the players bet number is between 0 and 6
            validate_bet_number(bet_number)?;

            // Check that only one denom was sent
            let coin = one_coin(&info).unwrap(); 
            validate_denom(&coin, &bankroll_balance)?; 

            validate_bet_amount(&coin)?; 

            validate_funds_sent(&coin, &info)?;

            validate_bet_vs_bankroll(&info, &bankroll_balance)?; 

            // Save the game state
            GAME.save(deps.storage, idx.into(), &game)?;

            Ok(Response::new()
            .add_attribute("game_idx", game.game_idx.to_string())
            .add_message(
                EntropyRequest {
                    callback_gas_limit,
                    callback_address: env.contract.address,
                    funds: vec![Coin {
                        denom: "ukuji".to_string(),
                        amount: Uint128::from(beacon_fee),
                    }],
                    callback_msg: EntropyCallbackData {
                        original_sender: info.sender,
                    },
                }
                .into_cosmos(beacon_addr)?,
            ))
        }

        ExecuteMsg::FreeSpin { bet_number } => {
            let state = STATE.load(deps.storage)?;
            let idx = IDX.load(deps.storage)?;
            let beacon_addr = state.entropy_beacon_addr;

            // How much gas our callback will use. This is an educated guess, so we usually want to overestimate.
            let callback_gas_limit = 150_000u64;

            let mut player_history = load_player_history_or_create_new(deps.storage, info.sender.clone().to_string())?; 
            if player_history.free_spins == Uint128::zero() {
                return Err(ContractError::NoFreeSpinsLeft {});
            }

            // The beacon allows us to query the fee it will charge for a request, given the gas limit we provide.
            let beacon_fee = calculate_beacon_fee(&deps, &info.sender.to_string(), callback_gas_limit)?; 

            let game = Game::new_game(
                &info.sender.to_string(),
                idx.into(),
                bet_number.into(),
                1u128,
            );

            // Check that the players bet number is between 0 and 6
            validate_bet_number(bet_number)?;

            update_player_history_and_save(deps.storage, info.sender.clone().to_string(), &mut player_history)?; 

            // Save the game state
            GAME.save(deps.storage, idx.into(), &game)?;

            Ok(Response::new()
                .add_attribute("game_type", "free_spin")
                .add_attribute("remaining_freespins", player_history.free_spins.to_string())
                .add_attribute("game_idx", game.game_idx.to_string())
                .add_message(
                    EntropyRequest {
                        callback_gas_limit,
                        callback_address: env.contract.address,
                        funds: vec![Coin {
                            denom: "ukuji".to_string(), // Change this to match your chain's native token.
                            amount: Uint128::from(beacon_fee),
                        }],
                        callback_msg: EntropyCallbackData {
                            original_sender: info.sender,
                        },
                    }
                    .into_cosmos(beacon_addr)?,
                ))
        }

        ExecuteMsg::ReceiveEntropy(data) => {

            // Load the game states from storage
            let state = STATE.load(deps.storage)?;
            let mut idx = IDX.load(deps.storage)?;
            let game = GAME.load(deps.storage, idx.into()).unwrap();

            // Get the address of the entropy beacon
            let beacon_addr = state.entropy_beacon_addr;

            // Verify that the callback was called by the beacon, and not by someone else.
            verify_callback_sender(
                &info.sender.to_string(), 
                &beacon_addr.to_string(), 
                &data.requester.to_string(), 
                &env.contract.address.to_string()
            )?;

            // The callback data has 64 bytes of entropy, in a Vec<u8>.
            let entropy = data.entropy;

            // Calculate the game outcome based on the provided entropy and the game rule set
            let result = get_outcome_from_entropy(&entropy, &game.rule_set);
            let outcome = result.clone(); 

            // Load or initialize the player's history
            let mut player_history = match PLAYER_HISTORY.may_load(deps.storage, game.player.clone().to_string()) {
                Ok(Some(player_history)) => player_history,
                Ok(None) => PlayerHistory::new(game.player.clone()),
                Err(_) => return Err(ContractError::UnableToLoadPlayerHistory {player_addr: info.sender.to_string()}),
            };

            // Process the game result, updating game state, player history, and index as needed
            let response = process_game_result(
                deps.storage,
                &outcome, 
                &game, 
                &mut player_history, 
                &mut idx, 
                )?; 

            // Return the response containing the outcome of the game and any other relevant information
            Ok(response)
        }

    }
}

// Entry point for query messages to the contract 
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    // Match the incomming query message aginst the supported query types 
    match msg {
        // If msg is a request for game information at a specific index 
        QueryMsg::Game { idx } => {
            // Load the game state with the provided index
            // If game idx is not found, handle error gracefully and return a custom error message
            let game = GAME.load(deps.storage, idx.into())
                .map_err(|_| ContractError::GameNotFound(idx))?; // Handle the error gracefully

            // Serialize the game response into a binary format 
            // if error occurs during serialization, handle error gracefully and return a custom error
            to_binary(&GameResponse {
                idx: game.game_idx.into(),
                player: game.player,
                bet_number: game.bet_number.into(),
                bet_size: game.bet_size.into(),
                played: game.played,
                game_outcome: game.outcome,
                win: game.win,
                payout: game.payout,
            }).map_err(|e| 
                ContractError::QueryError(format!("Serialization error: {}", e)))
        }

        QueryMsg::PlayerHistory { player_addr } => {
            // Load the player history state with the provided player address
            let player_history = PLAYER_HISTORY.load(deps.storage, player_addr.clone().to_string())
                .map_err(|_| ContractError::UnableToLoadPlayerHistory{player_addr: player_addr.to_string()})?;

            // Serialize the player history response into a binary format 
            to_binary(&PlayerHistory {
                player_address: player_addr.to_string(),
                games_played: player_history.games_played,
                wins: player_history.wins,
                losses: player_history.losses,
                total_coins_spent: player_history.total_coins_spent,
                total_coins_won: player_history.total_coins_won,
                free_spins: player_history.free_spins,
            }).map_err(|e| 
                ContractError::QueryError(format!("Serialization error: {}", e)))
        }

        QueryMsg::LatestGameIndex { } => {
            // Load the game index state
            // If game index is not found, handle error gracefully and return a custom error message
            let mut idx = IDX.load(deps.storage)
                .map_err(|_| ContractError::UnableToLoadGameIndex{})?;

            idx = idx - Uint128::from(1u128); // Decrement the index by 1 to get the last finished game index

            // Serialize the game index response into a binary format 
            // if error occurs during serialization, handle error gracefully and return a custom error
            to_binary(&LatestGameIndexResponse {
                idx: idx.into(),
            }).map_err(|e| 
                ContractError::QueryError(format!("Serialization error in LatestGameIndexResponse: {}", e)))
        }
    
        QueryMsg::LeaderBoard {  } => {
            let leaderboard = query_leaderboard(deps);
            Ok(to_binary(&leaderboard)?) 
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("action", "migrate"))
}