// use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{ 
    /*coins,*/ from_binary, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, //Empty,
    Env, MessageInfo, Response, StdResult, //SubMsg, SubMsgExecutionResponse, SubMsgResponse,
    Uint128,
};

// use cw20::{Cw20Coin, Cw20Contract, Cw20ExecuteMsg};
// use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
// use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use cw_utils::one_coin;
use entropy_beacon_cosmos::{/*beacon,*/ EntropyCallbackMsg, EntropyRequest};
use kujira::denom::Denom;

// use entropy_beacon_cosmos::beacon::CalculateFeeQuery; // <--- NEW ENTROPY STUFF

use crate::error::ContractError;
use crate::msg::{
    EntropyCallbackData, ExecuteMsg, GameResponse, InstantiateMsg, MigrateMsg,
    QueryMsg, /* ReceiveMsg,
             CreateMsg, DetailsResponse, ExecuteMsgEscrow as Cw20ExecuteMsgEscrow, */
};
use crate::state::{Game, RuleSet, State, GAME, IDX, STATE};

use num_bigint::BigUint;
use num_traits::cast::ToPrimitive;
// use rand::RngCore;
use rand::{rngs::OsRng, RngCore};
use sha3::{Digest, Sha3_512};

/// Our [`InstantiateMsg`] contains the address of the entropy beacon contract.
/// We save this address in the contract state.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let mut state = State {
        entropy_beacon_addr: msg.entropy_beacon_addr,
        owner_addr: Addr::unchecked("empty"),
        house_bankroll: Coin {
            denom: "USK".to_string(),
            amount: Uint128::zero(),
        },
        token: msg.token,
        play_amount: msg.play_amount,
        win_amount: msg.win_amount,
        fee_amount: msg.fee_amount,
        rule_set: msg.rule_set,
    };

    state.owner_addr = Addr::unchecked("owner");
    let _validated_owner_address: Addr = deps.api.addr_validate(&state.owner_addr.to_string())?;

    // Harpoon-4, Kujira Testnet
    state.entropy_beacon_addr =
        Addr::unchecked("kujira1xwz7fll64nnh4p9q8dyh9xfvqlwfppz4hqdn2uyq2fcmmqtnf5vsugyk7u");
    
    // Kaiyo-1, Mainnet
    //state.entropy_beacon_addr =
    //    Addr::unchecked("kujira1x623ehq3gqx9m9t8asyd9cgehf32gy94mhsw8l99cj3l2nvda2fqrjwqy5"); 
    
    let _validated_beacon_address: Addr = deps
        .api
        .addr_validate(&state.entropy_beacon_addr.to_string())?;

    STATE.save(deps.storage, &state)?;
    IDX.save(deps.storage, &Uint128::zero())?;
    Ok(Response::new()
    .add_attribute("method", "instantiate")
    .add_attribute("owner", state.owner_addr.to_string())
    .add_attribute("entropy_beacon_addr", state.entropy_beacon_addr.to_string())
    .add_attribute("token", state.token.to_string())
)

}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg.clone() {

        // #STEP 1: 
        // Validate player's bet amount and number
        // and handle requesting entropy from the beacon.
        ExecuteMsg::Pull {
            player_bet_amount, 
            player_bet_number} => {
                execute_entropy_beacon_pull(
                    deps, 
                    env, 
                    info, 
                    player_bet_amount, 
                    player_bet_number
                )
            }, 

        // #STEP 2: 
        // Handle receiving entropy from the beacon.
        ExecuteMsg::ReceiveEntropy(
            data) => { 
                execute_recieve_entropy(
                    deps, 
                    env, 
                    info, 
                    data
                )
            },

        // #STEP 3:
        // Handle player placing a bet and spinning the wheel
        ExecuteMsg::Spin {
            player_bet_amount,
            player_bet_number} => { 
                execute_spin(
                    deps, 
                    env, 
                    info, 
                    player_bet_amount, 
                    player_bet_number
                ) 
            },

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

pub fn execute_validate_bet(deps: &DepsMut, info: MessageInfo, player_bet_amount: Uint128, player_bet_number: u8
) -> bool {

    let state = STATE.load(deps.storage).unwrap();

    //TODO: How can i ensure that we avoid floating point numbers in this calculation?
    // Trying this: rounding up to nearest integer with checked_div
    if player_bet_amount > state.house_bankroll.amount.checked_div(Uint128::new(10)).unwrap() {
        return false
        // return Err(ContractError::InvalidBetAmount {});
    }

    // Check that the players bet number is between 0 and 6
    if player_bet_number > 6 {
        return false
        // return Err(ContractError::InvalidBetNumber {});
    }
    
    // Check that only one denom was sent
    // let coin = one_coin(&info).unwrap(); 
    let coin = match one_coin(&info) {
        Ok(coin) => coin,
        Err(_) => return false //return Err(ContractError::InvalidCoin {}),
    }; 

    // Check that the denom is the same as the token in the state
    let state = STATE.load(deps.storage).unwrap();
    if Denom::from(coin.denom) != state.token {
        return false
        // return Err(ContractError::InvalidToken {});
    }

    // Check that the amount is the same as the play_amount in the state
    if coin.amount != player_bet_amount || player_bet_amount < Uint128::from(1u128) {
        return false
        // return Err(ContractError::InsufficientFunds {});
    }

    // Check that the player_bet_number is between 0 and 6
    if player_bet_number > 6 {
        return false
        // return Err(ContractError::InvalidBetNumber {});
    }

    // Check that the player_bet_amount does not exceed 10% of house bankroll
    let house_bankroll = state.house_bankroll.amount;
    if player_bet_amount > house_bankroll / Uint128::from(10u128) {
        return false
        // return Err(ContractError::InvalidBetAmount {});
    }

    true 
}


pub fn execute_spin(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    player_bet_amount: Uint128,
    player_bet_number: u8,
) -> Result<Response, ContractError> {
    {
        let state = STATE.load(deps.storage).unwrap();
        let coin = one_coin(&info).unwrap(); // Check that only one denom was sent

        // Check that the denom is the same as the token in the state
        if Denom::from(coin.denom.clone()) != state.token {
            return Err(ContractError::InvalidToken {});
        }
        // Check that the amount is the same as the play_amount in the state
        if coin.amount.clone() != player_bet_amount || player_bet_amount < Uint128::from(1u128) {
            return Err(ContractError::InsufficientFunds {});
        }
        // Get the current game index
        let idx = IDX.load(deps.storage)?;

        // Check if there is a game at the current index
        let game = GAME.may_load(deps.storage, idx.u128())?;

        // Collect the players bet and send it to the owner contract address 
        let send_msg = BankMsg::Send {
            to_address: state.owner_addr.to_string(),
            amount: vec![coin],
        };

        // Check if the game has been played
        match game {
            // The game has been played
            Some(mut game) => {
                // only let players access their own game
                if game.player.clone() != info.sender {
                    return Err(ContractError::Unauthorized {});
                }

                // Get the result of the game
                let outcome = game.result.clone().unwrap();

                // Calculate the payout
                let calculated_payout =
                    calculate_payout(player_bet_amount, outcome[0], state.rule_set.clone());

                // Return the payout to the player if they win
                if game.win(player_bet_number) {
                    let payout_coin = Coin {
                        denom: state.token.to_string().clone(),
                        amount: calculated_payout,
                    };
                    // Player has won, update game state
                    game.win = Some(true);

                    // Send the payout to the player
                    let payout_msg = BankMsg::Send {
                        to_address: game.player.to_string(),
                        amount: vec![payout_coin],
                    };                    

                    // generate the response
                    let response = Response::new()
                        .add_attribute("game", idx.u128().to_string())
                        .add_attribute("player", game.player.clone())
                        .add_attribute("result", "win")
                        .add_attribute("payout", calculated_payout.to_string())
                        .add_message(send_msg.clone()) // Add the collection message to the response //TODO: Can i add two messages to the response? 
                        .add_message(payout_msg.clone()); // Add the payout message to the response
                        

                    // Increment gameID
                    IDX.save(deps.storage, &(idx + Uint128::from(1u128)))?;

                    // Player has won, update game state
                    game.played = true;
                    game.bet = player_bet_number;
                    game.payout = calculated_payout;
                    game.result = Some(outcome);
                    GAME.save(deps.storage, idx.u128(), &game)?;

                    Ok(response.add_message(payout_msg).into())
                } else {
                    // Increment gameID
                    IDX.save(deps.storage, &(idx + Uint128::from(1u128)))?;

                    // Player has lost, update game state
                    game.played = true;
                    game.win = Some(false);
                    game.bet = player_bet_number;
                    game.payout = Uint128::zero();
                    game.result = Some(outcome);
                    GAME.save(deps.storage, idx.u128(), &game)?;
                    return Ok(Response::new()
                        .add_attribute("game", idx.u128().to_string())
                        .add_attribute("player", game.player.to_string())
                        .add_attribute("result", "lose"));
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
                    win: None,
                };

                GAME.save(deps.storage, idx.u128(), &game)?; // Save the game

                Ok(Response::new()
                    .add_attribute("game", idx.u128().to_string())
                    .add_attribute("player", game.player)
                    .add_attribute("result", "pending"))
            }
        }
    }
}

pub fn execute_recieve_entropy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    data: EntropyCallbackMsg,
) -> Result<Response, ContractError> {
    // Load the game state from the contract
    let state = STATE.load(deps.storage)?;

    // Get the address of the entropy beacon
    let beacon_addr = state.entropy_beacon_addr;

    // IMPORTANT: Verify that the callback was called by the beacon, and not by someone else.
    if info.sender != beacon_addr {
        return Err(ContractError::Unauthorized {});
    }

    //* IMPORTANT: Verify that the original requester for entropy is trusted (e.g.: this contract)
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

    return Ok(Response::new()
        .add_attribute("game", callback_data.game)
        .add_attribute("player", game.player)
        .add_attribute("result", "pending"));
}

pub fn execute_entropy_beacon_pull(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // _msg: ExecuteMsg,
    _player_bet_amount: Uint128,
    player_bet_number: u8,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // Check that players bet amount is <= 10% of the house bankroll amount 
    if !execute_validate_bet(
    &deps, 
    info.clone(), 
    _player_bet_amount, 
    player_bet_number) {
    return Err(ContractError::InvalidBet {});
    }

    // Checks that only one denom is sent
    let coin = one_coin(&info)?;  

    // Check that the denom is the same as the denom of the house bankroll, and that the amount is the same as the bet amount 
    if Denom::from(coin.denom) != state.token || coin.amount != state.play_amount {
        return Err(ContractError::InsufficientFunds {});
    }

    // Get the current gameID
    let idx = IDX.load(deps.storage)?;

    // Load the game state from the contract -- UNESSESSARY? WILL BE OVERWRITTEN
    // let game = GAME.load(deps.storage, idx.u128())?;

    // Create a new game state
    let game = Game {
        player: info.sender.clone(),
        bet: player_bet_number,
        payout: Uint128::zero(), // Payout not yet decided in this step 
        result: None,
        played: false,
        win: None,
    };

    // Save the game state to the contract
    GAME.save(deps.storage, idx.u128(), &game)?;

    // Create a request for entropy from the Beacon contract 
    let mut msgs = vec![EntropyRequest {
        callback_gas_limit: 100_000u64,
        callback_address: env.contract.address,
        funds: vec![Coin {
            denom: state.token.to_string(),
            amount: state.play_amount,
        }],
        callback_msg: EntropyCallbackData {
            original_sender: info.sender,
            game: idx,
        },
    }
    .into_cosmos(state.entropy_beacon_addr)?];

    // If there is a fee, send it to the fee address
    if !state.fee_amount.is_zero() {
        msgs.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: kujira::utils::fee_address().to_string(),
            amount: state.token.coins(&state.fee_amount),
        }))
    };

    // Response to the contract caller
    Ok(Response::new()
        .add_attribute("game", idx)
        .add_attribute("player", game.player)
        .add_messages(msgs))
}


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

pub fn generate_entropy_test() -> [u8; 64] {
    let mut entropy = [0u8; 64];
    let mut rng = OsRng;
    rng.fill_bytes(&mut entropy);
    entropy
}