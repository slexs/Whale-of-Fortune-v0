#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, Uint128};
use cw2::set_contract_version;

use cw_utils::one_coin;
use entropy_beacon_cosmos::EntropyRequest;

use crate::error::ContractError;
use crate::msg::{EntropyCallbackData, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, GameResponse};

use crate::state::{Game, PlayerHistory, State, GAME, IDX, PLAYER_HISTORY, STATE, LatestGameIndexResponse};

use crate::helpers::{
    calculate_beacon_fee, get_bankroll_balance, get_outcome_from_entropy, get_rule_set,
    load_player_history_or_create_new, process_game_result, query_leaderboard,
    update_player_history_and_save, validate_bet_amount, validate_bet_number,
    validate_bet_vs_bankroll, validate_denom, validate_funds_sent,
    validate_sent_amount_to_cover_fee, verify_callback_sender,
};

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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let validated_entropy_beacon_addr = deps.api.addr_validate(msg.entropy_beacon_addr.as_ref())?;

    let state = State {
        admin: deps.api.addr_validate(_info.sender.as_ref())?,
        entropy_beacon_addr: validated_entropy_beacon_addr,
        free_spin_threshold: Uint128::new(5),
        house_bankroll: Coin {
            // Initialize house bankroll to 0
            denom: "ukuji".to_string(),
            amount: Uint128::zero(),
        },
    };

    STATE.save(deps.storage, &state)?;
    IDX.save(deps.storage, &Uint128::zero())?;

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

            // Check that only one denom was sent
            let coin = one_coin(&info).unwrap();

            let sent_amount = coin.amount;

            // How much gas our callback will use. This is an educated guess, so we usually want to overestimate.
            let callback_gas_limit = 150_000u64;

            // The beacon allows us to query the fee it will charge for a request, given the gas limit we provide.
            let beacon_fee = calculate_beacon_fee(&deps, info.sender.as_ref(), callback_gas_limit)?;

            validate_sent_amount_to_cover_fee(sent_amount, beacon_fee)?;

            let rule_set = get_rule_set();

            // Get the balance of the house bankroll (contract address balance)
            let bankroll_balance =
                get_bankroll_balance(&deps, env.contract.address.as_ref(), "ukuji".to_string())?;

            // Check that the players bet number is between 0 and 6
            validate_bet_number(bet_number)?;

            validate_denom(&coin, &bankroll_balance)?;

            validate_bet_amount(&coin)?;

            validate_funds_sent(&coin, &info)?;

            validate_bet_vs_bankroll(&info, &bankroll_balance)?;

            // Create a new game state
            let mut game = Game::new_game(
                info.sender.as_ref(),
                idx.into(),
                bet_number.into(),
                sent_amount.into(),
            );

            game.rule_set = rule_set;

            // Save the game state
            GAME.save(deps.storage, idx.into(), &game)?;

            let entropy_request = EntropyRequest {
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
            .into_cosmos(beacon_addr)?;

            Ok(Response::new()
                .add_attribute("game_idx", game.game_idx.to_string())
                .add_message(entropy_request))
        }

        ExecuteMsg::FreeSpin { bet_number } => {
            let state = STATE.load(deps.storage)?;
            let idx = IDX.load(deps.storage)?;
            let beacon_addr = state.entropy_beacon_addr;

            // How much gas our callback will use. This is an educated guess, so we usually want to overestimate.
            let callback_gas_limit = 150_000u64;

            let mut player_history =
                load_player_history_or_create_new(deps.storage, info.sender.to_string())?;
            if player_history.free_spins == Uint128::zero() {
                return Err(ContractError::NoFreeSpinsLeft {});
            }

            // The beacon allows us to query the fee it will charge for a request, given the gas limit we provide.
            let beacon_fee = calculate_beacon_fee(&deps, info.sender.as_ref(), callback_gas_limit)?;

            // Check that the players bet number is between 0 and 6
            validate_bet_number(bet_number)?;
            update_player_history_and_save(
                deps.storage,
                info.sender.to_string(),
                &mut player_history,
            )?;

            let game = Game::new_game(info.sender.as_ref(), idx.into(), bet_number.into(), 1u128);

            // Save the game state
            GAME.save(deps.storage, idx.into(), &game)?;

            let entropy_request = EntropyRequest {
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
            .into_cosmos(beacon_addr)?;

            Ok(Response::new()
                .add_attribute("game_type", "free_spin")
                .add_attribute("remaining_freespins", player_history.free_spins.to_string())
                .add_attribute("game_idx", game.game_idx.to_string())
                .add_message(entropy_request))
        }

        ExecuteMsg::ReceiveEntropy(data) => {
            // Load the game states from storage
            let state = STATE.load(deps.storage)?;
            let mut idx = IDX.load(deps.storage)?;
            let game = GAME.load(deps.storage, idx.into()).unwrap();

            // Verify that the callback was called by the beacon, and not by someone else.
            verify_callback_sender(
                &info.sender.to_string(),
                &state.entropy_beacon_addr.to_string(),
                &data.requester.to_string(),
                &env.contract.address.to_string(),
            )?;

            // The callback data has 64 bytes of entropy, in a Vec<u8>.
            let entropy = data.entropy;

            // Calculate the game outcome based on the provided entropy and the game rule set
            let result = get_outcome_from_entropy(&entropy, &game.rule_set);

            // Load or initialize the player's history
            let mut player_history =
                match PLAYER_HISTORY.may_load(deps.storage, game.player.clone()) {
                    Ok(Some(player_history)) => player_history,
                    Ok(None) => PlayerHistory::new(game.player.clone()),
                    Err(_) => {
                        return Err(ContractError::UnableToLoadPlayerHistory {
                            player_addr: info.sender.to_string(),
                        })
                    }
                };

            // Process the game result, updating game state, player history, and index as needed
            let response =
                process_game_result(deps.storage, &result, &game, &mut player_history, &mut idx)?;

            // Return the response containing the outcome of the game and any other relevant information
            Ok(response)
        }

        ExecuteMsg::AdminExecuteChangeEntropyAddr { new_addr } => {
            let mut state = STATE.load(deps.storage)?;

            // Only the contract admin can change the entropy address
            if info.sender != state.admin {
                Err(ContractError::UnauthorizedNotAdmin {
                    addr: info.sender.to_string(),
                })
            } else {
                state.entropy_beacon_addr = new_addr.clone();
                STATE.save(deps.storage, &state)?;
                Ok(Response::new()
                    .add_attribute("action", "change_entropy_addr")
                    .add_attribute("new_addr", new_addr.to_string()))
            }
        }

        ExecuteMsg::AdminExecuteChangeFreeSpinThreshold { new_threshold } => {
            let mut state = STATE.load(deps.storage)?;

            // Only the contract admin can change the free spin threshold
            if info.sender != state.admin {
                Err(ContractError::UnauthorizedNotAdmin {
                    addr: info.sender.to_string(),
                })
            } else {
                state.free_spin_threshold = new_threshold;
                STATE.save(deps.storage, &state)?;
                Ok(Response::new()
                    .add_attribute("action", "change_free_spin_threshold")
                    .add_attribute("new_threshold", new_threshold.to_string()))
            }
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
            let game = GAME
                .load(deps.storage, idx.into())
                .map_err(|_| ContractError::GameNotFound(idx))?; // Handle the error gracefully

            // Serialize the game response into a binary format
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
            let player_history = PLAYER_HISTORY
                .load(deps.storage, player_addr.to_string())
                .map_err(|_| ContractError::UnableToLoadPlayerHistory {
                    player_addr: player_addr.to_string(),
                })?;

            // Serialize the player history response into a binary format
            to_binary(&player_history)
                .map_err(|e| ContractError::QueryError(format!("Serialization error: {}", e)))
        }

        QueryMsg::LatestGameIndex {} => {
            // Load the game index state
            // If game index is not found, handle error gracefully and return a custom error message
            let mut idx = IDX
                .load(deps.storage)
                .map_err(|_| ContractError::UnableToLoadGameIndex {})?;

            idx -= Uint128::from(1u128); // Decrement the index by 1 to get the last finished game index

            // Serialize the game index response into a binary format
            to_binary(&LatestGameIndexResponse {
                idx: idx.into(),
            }).map_err(|e| {
                ContractError::QueryError(format!(
                    "Serialization error in LatestGameIndexResponse: {}",
                    e
                ))
            })
        }

        QueryMsg::LeaderBoard {} => {
            let leaderboard = query_leaderboard(deps);
            to_binary(&leaderboard)
                .map_err(|e| ContractError::QueryError(format!("Serialization error: {}", e)))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("action", "migrate"))
}
