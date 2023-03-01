// use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, Coin, 
};

// use cw_storage_plus::{Item, Map};
use cw_utils::one_coin;
use entropy_beacon_cosmos::EntropyRequest;
use kujira::denom::Denom;

use crate::error::ContractError;
use crate::msg::{
    EntropyCallbackData, ExecuteMsg, GameResponse, InstantiateMsg, MigrateMsg, QueryMsg
};
use crate::state::{State, STATE, Game, GAME, IDX, RuleSet};

// use rand::{Rng, RngCore};
use rand::RngCore;
use num_bigint::BigUint;
use sha3::{Digest, Sha3_512};
use num_traits::cast::ToPrimitive;

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
        entropy_beacon_addr: msg.entropy_beacon_addr,
        token: msg.token,
        play_amount: msg.play_amount,
        win_amount: msg.win_amount,
        fee_amount: msg.fee_amount,
        rule_set: msg.rule_set,
    };

    STATE.save(deps.storage, &state)?;
    IDX.save(deps.storage, &Uint128::zero())?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
    player_bet_amount: Uint128,
    player_bet_number: u8,  
) -> Result<Response, ContractError> {
    match msg {

        // Handle player placing a bet and spinning the wheel 
        ExecuteMsg::Spin{ bet_amount: Uint128} => {

            let state = STATE.load(deps.storage)?;
            let coin = one_coin(&info)?; // Check that only one denom was sent 

            // Coin denom and amount 
            if Denom::from(coin.denom) != state.token || coin.amount != player_bet_amount {
                return Err(ContractError::InsufficientFunds {});
            }

            /* 
            if Denom::from(coin.denom) != state.token || coin.amount != state.play_amount {
                return Err(ContractError::InsufficientFunds {});
            }
            */

            let idx = IDX.load(deps.storage)?; // Get the current game index
            let mut game = GAME.may_load(deps.storage, idx.u128())?; // Check if there is a game at the current index

            // Check if the game has been played 
            match game {
                Some(mut game) => {
                    if game.player != info.sender {
                        return Err(ContractError::Unauthorized {});
                    }

                    let outcome = game.result.unwrap(); // Get the result of the game
                    let calculated_payout = calculate_payout(player_bet_amount, outcome[0], state.rule_set.clone()); // Calculate the payout
                
                    // Update game with payout and save it 
                    game.bet = player_bet_number; 
                    game.payout = calculated_payout; 
                    game.result = Some(outcome); 

                    // Save the game
                    GAME.save(deps.storage, idx.u128(), &game)?; 

                    // Return the payout to the player if they win 
                    if game.win(player_bet_number) {
                        let payout_amount = calculated_payout; 
                        let payout_coin = Coin {
                            denom: state.token.to_string().clone(), 
                            amount: payout_amount, 
                        }; 

                        let payout_msg = CosmosMsg::Bank(BankMsg::Send {
                            to_address: game.player.to_string(),
                            amount: vec![payout_coin],
                        }); 


                        let response = Response::new()
                        .add_attribute("game", idx.u128().to_string())
                        .add_attribute("player", game.player)
                        .add_attribute("result", "win")
                        .add_attribute("payout", calculated_payout.to_string());

                        // response.add_message(payout_msg);

                        Ok(response.add_message(payout_msg).into()) 

                    } else {
                        return Ok(Response::new()
                                .add_attribute("game", idx.u128().to_string())
                                .add_attribute("player", game.player)
                                .add_attribute("result", "lose"))
                    }

                }

                None => {
                    // Game has not been played yet 
                    let game = Game {
                        player: info.sender.clone(),
                        bet: player_bet_number,
                        payout: Uint128::zero(),
                        result: None,
                        played: false,
                    }; 

                    GAME.save(deps.storage, idx.u128(), &game)?; // Save the game

                    Ok(Response::new()
                        .add_attribute("game", idx.u128().to_string())
                        .add_attribute("player", game.player)
                        .add_attribute("result", "pending"))
                }
            }
        }

        // Here we handle requesting entropy from the beacon.
        ExecuteMsg::Pull {} => {
            let state = STATE.load(deps.storage)?;
            let coin = one_coin(&info)?;
            if Denom::from(coin.denom) != state.token || coin.amount != state.play_amount {
                return Err(ContractError::InsufficientFunds {});
            }
            let idx = IDX.load(deps.storage)?;

            let game = GAME.load(deps.storage, idx.u128())?;

            let game = Game {
                player: info.sender.clone(),
                bet: player_bet_number, 
                payout: game.payout,
                result: None,
                played: false,
            };

            GAME.save(deps.storage, idx.u128(), &game)?;
            IDX.save(deps.storage, &(idx + Uint128::one()))?;

            let mut msgs = vec![EntropyRequest {
                callback_gas_limit: 100_000u64,
                callback_address: env.contract.address,
                funds: vec![],
                callback_msg: EntropyCallbackData {
                    original_sender: info.sender,
                    game: idx,
                },
            }
            .into_cosmos(state.entropy_beacon_addr)?];

            if !state.fee_amount.is_zero() {
                msgs.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: kujira::utils::fee_address().to_string(),
                    amount: state.token.coins(&state.fee_amount),
                }))
            };

            Ok(Response::new()
                .add_attribute("game", idx)
                .add_attribute("player", game.player)
                .add_messages(msgs))
        }

        // Here we handle receiving entropy from the beacon.
        ExecuteMsg::ReceiveEntropy(data) => {
            let state = STATE.load(deps.storage)?;
            let beacon_addr = state.entropy_beacon_addr;

            // IMPORTANT: Verify that the callback was called by the beacon, and not by someone else.
            if info.sender != beacon_addr {
                return Err(ContractError::Unauthorized {});
            }

            // IMPORTANT: Verify that the original requester for entropy is trusted (e.g.: this contract)
            if data.requester != env.contract.address {
                return Err(ContractError::Unauthorized {});
            }

            // The callback data has 64 bytes of entropy, in a Vec<u8>.
            let entropy = data.entropy;

            // We can parse out our custom callback data from the message.
            let callback_data = data.msg;
            let callback_data: EntropyCallbackData = from_binary(&callback_data)?;
            let mut game = GAME.load(deps.storage, callback_data.game.u128())?;

            // gets a result (0-6) from the entropy, and sets game state to played
            game.result = Some(get_outcome_from_entropy(&entropy));
            game.played = true; 

            GAME.save(deps.storage, callback_data.game.u128(), &game)?;

            Ok(Response::new()
            .add_attribute("game", callback_data.game)
            .add_attribute("player", game.player)
            .add_attribute("result", "pending")) 
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Game { idx } => {
            let game = GAME.load(deps.storage, idx.u128())?;
            to_binary(&GameResponse {
                idx,
                player: game.player.clone(),
                result: game.result.as_ref().map(|x| x.clone().into()),
                win: game.win(game.bet),
            })
        }
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    let mut config = STATE.load(deps.storage)?;
    config.fee_amount = msg.fee_amount;
    STATE.save(deps.storage, &config)?;

    Ok(Response::new())
}

fn calculate_payout(bet_amount: Uint128, result: u8, rule_set: RuleSet) -> Uint128 {
    match result {
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

pub fn get_outcome_from_entropy(entropy: &[u8]) -> Vec<u8> {
    // let entropy_array: [u8; 64] = entropy.try_into().unwrap();

    // Convert 64-byte entropy to 512-bit integer
    let mut rng = rand::rngs::OsRng {};
    let mut seed = [0u8; 64];
    rng.fill_bytes(&mut seed);
    let mut hasher = Sha3_512::new();
    hasher.update(&seed);
    hasher.update(entropy);
    let result = hasher.finalize();

    // Divide result by 7 and take the remainder to get outcome (0-6)
    let num = BigUint::from_bytes_be(&result);
    let outcome = (num % BigUint::from(7u32)).to_u8().unwrap();
    vec![outcome]
}



