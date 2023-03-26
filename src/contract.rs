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
use crate::state::{Game, RuleSet, PlayerHistory, State, GAME, IDX, STATE, PLAYER_HISTORY};

use crate::helpers::{calculate_payout, /* execute_validate_bet ,*/ get_outcome_from_entropy};

// use cw_storage_plus::Map;

// version info for migration info
const CONTRACT_NAME: &str = "entropiclabs/Whale-of-fortune-v1.0.1";
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
        // Here we handle requesting entropy from the beacon.
        ExecuteMsg::Spin { bet_number } => {
            let state = STATE.load(deps.storage)?;
            let beacon_addr = state.entropy_beacon_addr;

            // Note: In production you should check the denomination of the funds to make sure it matches the native token of the chain.
            let sent_amount: Uint128 = info.funds.iter().map(|c| c.amount).sum();

            // How much gas our callback will use. This is an educated guess, so we usually want to overestimate.
            // IF YOU ARE USING THIS CONTRACT AS A TEMPLATE, YOU SHOULD CHANGE THIS VALUE TO MATCH YOUR CONTRACT.
            // If you set this too low, your contract will fail when receiving entropy, and the request will NOT be retried.
            let callback_gas_limit = 100_000u64;

            // The beacon allows us to query the fee it will charge for a request, given the gas limit we provide.
            let beacon_fee =
                CalculateFeeQuery::query(deps.as_ref(), callback_gas_limit, beacon_addr.clone())?;

            // Check if the user sent enough funds to cover the fee.
            if sent_amount < Uint128::from(beacon_fee) {
                return Err(ContractError::InsufficientFunds {});
            }

            let idx = IDX.load(deps.storage)?;

            // Create a new game state
            let game = Game {
                player: info.sender.to_string(),
                game_idx: idx.into(),
                bet_number: bet_number.into(),
                bet_size: sent_amount.into(),
                outcome: "Pending".to_string(),
                played: false,
                win: false,
                payout: Coin {
                    denom: "ukuji".to_string(),
                    amount: Uint128::zero(),
                },
                rule_set: RuleSet {
                        zero: Uint128::new(24),
                        one: Uint128::new(12),
                        two: Uint128::new(8),
                        three: Uint128::new(4),
                        four: Uint128::new(2),
                        five: Uint128::new(1),
                        six: Uint128::new(1),
                    },
                };

            // Get the balance of the house bankroll (contract address balance)
            let bankroll_balance = deps
                .querier
                .query_balance(env.contract.address.to_string(), "ukuji".to_string());

            if bankroll_balance.is_err() {
                return Err(ContractError::ValidateBetUnableToGetBankrollBalance {
                    addr: env.contract.address.to_string(),
                });
            }

            // unwrap the bankroll balance
            let bankroll_balance = bankroll_balance.unwrap();

            // Check that the players bet number is between 0 and 6
            if bet_number > Uint128::new(6) {
                return Err(ContractError::InvalidBetNumber {});
            }

            // Check that only one denom was sent
            let coin = match one_coin(&info)
            {
                Ok(coin) => coin,
                Err(_) => return Err(ContractError::ValidateBetInvalidDenom {}),
            }; 

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
            if coin.amount != info.funds[0].amount {
                return Err(ContractError::ValidateBetFundsSentMismatch {
                    player_sent_amount: coin.amount,
                    bet_amount: info.funds[0].amount,
                });
            }

            /* Make sure the player's bet_amount does not exceed 1% of house bankroll
            Ex: House Bankroll 1000, player bets 10, max player payout is 450 */
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

            // Save the game state
            GAME.save(deps.storage, idx.into(), &game)?;

            Ok(Response::new()
            .add_attribute("game_idx", game.game_idx.to_string())
            .add_message(
                EntropyRequest {
                    callback_gas_limit,
                    callback_address: env.contract.address,
                    funds: vec![Coin {
                        denom: "ukuji".to_string(), // Change this to match your chain's native token.
                        amount: Uint128::from(beacon_fee),
                    }],
                    // A custom struct and data we define for callback info.
                    // If you are using this contract as a template, you should change this to match the information your contract needs.
                    callback_msg: EntropyCallbackData {
                        original_sender: info.sender,
                    },
                }
                .into_cosmos(beacon_addr)?,
            ))
        }

        // Here we handle receiving entropy from the beacon.
        ExecuteMsg::ReceiveEntropy(data) => {
            // Load the game states from storage
            let state = STATE.load(deps.storage)?;
            let mut idx = IDX.load(deps.storage)?;
            let mut game = GAME.load(deps.storage, idx.into()).unwrap();
            let beacon_addr = state.entropy_beacon_addr;

            // Verify that the callback was called by the beacon, and not by someone else.
            if info.sender != beacon_addr {
                return Err(ContractError::CallBackCallerError { 
                    caller: info.sender.to_string(), 
                    expected: beacon_addr.to_string() 
                });
            }

            // Verify that the original requester for entropy is trusted (e.g.: this contract)
            if data.requester != env.contract.address {
                return Err(ContractError::EntropyRequestError {
                    requester: data.requester.to_string(),
                    trusted: env.contract.address.to_string(),
                });
            }

            // The callback data has 64 bytes of entropy, in a Vec<u8>.
            let entropy = data.entropy;

            // We can parse out our custom callback data from the message.
            let callback_data = data.msg;
            let _callback_data = from_binary::<EntropyCallbackData>(&callback_data)?;

            // Get the calculated outcome from the entropy
            let result = Some(get_outcome_from_entropy(&entropy, &game.rule_set));
            
            // Check if result is None, throw error if so 
            if result.is_none() {
                return Err(ContractError::InvalidEntropyResult { result: format!("{:?}", result) });
            }

            // Unwrap the result
            let outcome = result.unwrap();

            // Check if result is empty, throw error if so
            if outcome.is_empty() {
                return Err(ContractError::InvalidEntropyResult { result: format!("{:?}", outcome) });
            }

            // Set the outcome in the game state
            game.outcome = outcome[0].to_string();
            GAME.save( deps.storage, idx.into(), &game)?;

             // Check if player history exists for this player: if not, create a new instance of it and save it, if err throw message
             let mut player_history = match PLAYER_HISTORY.may_load(deps.storage, info.sender.clone().to_string()) {
                Ok(Some(player_history)) => player_history,
                Ok(None) => PlayerHistory {
                    player_address: info.sender.clone().to_string(),
                    games_played: Uint128::zero(),
                    wins: Uint128::zero(),
                    losses: Uint128::zero(),
                    win_loss_ratio: Uint128::zero(),
                    total_coins_spent: Coin {
                        denom: "ukuji".to_string(),
                        amount: Uint128::zero(),
                    },
                    total_coins_won: Coin {
                        denom: "ukuji".to_string(),
                        amount: Uint128::zero(),
                    },
                },
                Err(_) => return Err(ContractError::UnableToLoadPlayerHistory {player_addr: info.sender.to_string()}),
            };

            // Check if player has won
            if game.is_winner(game.bet_number.into(), outcome.clone()) {
                                
                // Set game result flag
                game.win = true;
                game.played = true;
                game.outcome = outcome[0].to_string();

                // Calculate the player's payout
                let calculated_payout = calculate_payout(
                    game.bet_size.clone().into(),
                    outcome[0],
                    game.rule_set.clone(),
                );

                // Create payout coin
                let payout_coin = Coin {
                    denom: "ukuji".to_string(),
                    amount: calculated_payout,
                };

                // Set payout in game state
                game.payout = payout_coin.clone(); 

                // Create payout message, send payout to player 
                let _payout_msg = BankMsg::Send {
                    to_address: game.player.to_string(),
                    amount: vec![payout_coin],
                };

                // Save the game state 
                GAME.save(deps.storage, idx.into(), &game)?;

                // Update the player's history state
                player_history.games_played += Uint128::new(1);
                player_history.wins += Uint128::new(1);
                player_history.total_coins_spent = Coin{amount: Uint128::new(game.bet_size), denom: "ukuji".to_string()};
                player_history.total_coins_won = Coin{ amount: calculated_payout, denom: "ukuji".to_string() };
                PLAYER_HISTORY.save(deps.storage, game.player.to_string(), &player_history)?;
                
                // Increment and save the game index state for the next game
                idx += Uint128::new(1);
                IDX.save(deps.storage, &idx)?;

                return Ok(Response::new()
                    .add_message(_payout_msg)
                    .add_attribute("game_result", game.win.to_string())
                    .add_attribute("game_outcome", game.outcome)
                    .add_attribute("game_payout", calculated_payout.to_string()));
            } else {

                // Set game result flag 
                game.win = false;
                game.played = true;
                game.outcome = outcome[0].to_string();

                // Save the game state 
                GAME.save(deps.storage, idx.into(), &game)?;
                
                // Update the player's history state
                player_history.games_played += Uint128::new(1);
                player_history.losses += Uint128::new(1);
                player_history.total_coins_spent = Coin {amount: Uint128::new(game.bet_size), denom: "ukuji".to_string()};
                PLAYER_HISTORY.save(deps.storage, game.player.to_string(), &player_history)?;
                
                // Increment and save the game index state for the next game
                idx += Uint128::new(1);
                IDX.save(deps.storage, &idx)?;

                return Ok(Response::new()
                    .add_attribute("game_result", game.win.to_string())
                    .add_attribute("game_outcome", game.outcome)
                    .add_attribute("game_payout", Uint128::new(0).to_string()));
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

        // If msg is a request for player history information
        QueryMsg::PlayerHistory { player_addr } => {
            // Load the player history state with the provided player address
            // If player history is not found, handle error gracefully and return a custom error message
            let player_history = PLAYER_HISTORY.load(deps.storage, player_addr.clone().to_string())
                .map_err(|_| ContractError::UnableToLoadPlayerHistory{player_addr: player_addr.to_string()})?;

            // Calculate the player's win loss ratio, if loss is zero, set win loss ratio to wins to avoid div by zero error 
            let win_loss_ratio = if player_history.losses == Uint128::zero() {
                player_history.wins
            } else {
                player_history.wins.checked_div(player_history.losses).unwrap()
            }; 

            // Serialize the player history response into a binary format 
            // if error occurs during serialization, handle error gracefully and return a custom error
            to_binary(&PlayerHistory {
                player_address: player_addr.to_string(),
                games_played: player_history.games_played,
                wins: player_history.wins,
                losses: player_history.losses,
                win_loss_ratio: win_loss_ratio, 
                total_coins_spent: player_history.total_coins_spent,
                total_coins_won: player_history.total_coins_won,
            }).map_err(|e| 
                ContractError::QueryError(format!("Serialization error: {}", e)))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("action", "migrate"))
}
