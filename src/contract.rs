// use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, Addr,
};

// use cw_storage_plus::{Item, Map};
use cw_utils::one_coin;
use entropy_beacon_cosmos::{EntropyRequest, EntropyCallbackMsg, beacon};
use kujira::denom::Denom;

use entropy_beacon_cosmos::beacon::CalculateFeeQuery; // <--- NEW ENTROPY STUFF 

use crate::error::ContractError;
use crate::msg::{
    EntropyCallbackData, ExecuteMsg, GameResponse, InstantiateMsg, MigrateMsg, QueryMsg,
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

    // Harpoon-4 Kujira Testnet
    state.entropy_beacon_addr = Addr::unchecked("kujira1xwz7fll64nnh4p9q8dyh9xfvqlwfppz4hqdn2uyq2fcmmqtnf5vsugyk7u"); 
    let _validated_beacon_address: Addr = deps.api.addr_validate(&state.entropy_beacon_addr.to_string())?;

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
    match msg.clone() {
        // Handle player placing a bet and spinning the wheel
        ExecuteMsg::Spin {
            bet_amount: Uint128,
        } => execute_spin(deps, env, info, player_bet_amount, player_bet_number),

        // Here we handle requesting entropy from the beacon.
        ExecuteMsg::Pull {} 
            => execute_entropy_beacon_pull(deps, env, info,  player_bet_amount, player_bet_number),

        // Here we handle receiving entropy from the beacon.
        ExecuteMsg::ReceiveEntropy(data)    
            => execute_recieve_entropy( deps, env, info, data),
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

fn execute_spin(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    player_bet_amount: Uint128,
    player_bet_number: u8,
    ) -> Result<Response, ContractError> { {
    
        let state = STATE.load(deps.storage).unwrap();
        let coin = one_coin(&info).unwrap(); // Check that only one denom was sent

        // Check that the denom is the same as the token in the state
        if Denom::from(coin.denom) != state.token {
            return Err(ContractError::InvalidToken  {});
        }
        // Check that the amount is the same as the play_amount in the state
        if coin.amount != player_bet_amount || player_bet_amount < Uint128::from(1u128) {
            return Err(ContractError::InsufficientFunds {});
        }
        // Get the current game index
        let idx = IDX.load(deps.storage)?;

        // Check if there is a game at the current index
        let game = GAME.may_load(deps.storage, idx.u128())?; 

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

                // Update game with payout and save it -- This should be done after we've decided win? 
                // game.bet = player_bet_number;
                // game.payout = calculated_payout;
                // game.result = Some(outcome);

                // Save the game
                // GAME.save(deps.storage, idx.u128(), &game)?;

                // Return the payout to the player if they win
                if game.win(player_bet_number) {
                    let payout_coin = Coin {
                        denom: state.token.to_string().clone(),
                        amount: calculated_payout,
                    };
                    // Player has won, update game state
                    game.win = Some(true);

                    let payout_msg = CosmosMsg::Bank(BankMsg::Send {
                        to_address: game.player.to_string(),
                        amount: vec![payout_coin],
                    });

                    let response = Response::new()
                        .add_attribute("game", idx.u128().to_string())
                        .add_attribute("player", game.player.clone())
                        .add_attribute("result", "win")
                        .add_attribute("payout", calculated_payout.to_string());

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

fn execute_recieve_entropy(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        data: EntropyCallbackMsg,
    ) -> Result<Response, ContractError> {
        // Load the game state from the contract
        let state = STATE.load(deps.storage)?;
        // 
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
    
        return Ok(Response::new()
            .add_attribute("game", callback_data.game)
            .add_attribute("player", game.player)
            .add_attribute("result", "pending"))
    }


fn execute_entropy_beacon_pull(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // _msg: ExecuteMsg,
    _player_bet_amount: Uint128,
    player_bet_number: u8,
) -> Result<Response, ContractError> {

    let state = STATE.load(deps.storage)?;


    // let coin = one_coin(&info)?; -- Would only allow one coin? 
    let coin = Coin {
        denom: info.funds[0].denom.to_string(),
        amount: info.funds[0].amount,
    };

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
        win: None, 
    };

    // Save the game state to the contract
    GAME.save(deps.storage, idx.u128(), &game)?;

    // Save and increment the game index
    // DONT INCREMENT GAME ID UNTIL GAME IS COMPLETE 
    // IDX.save(deps.storage, &(idx + Uint128::one()))?;

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



fn calculate_payout(bet_amount: Uint128, outcome: u8, rule_set: RuleSet) -> Uint128 {
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

fn generate_entropy_test() -> [u8; 64] {
    let mut entropy = [0u8; 64];
    let mut rng = OsRng;
    rng.fill_bytes(&mut entropy);
    entropy
}


mod tests {
    use std::ops::Mul;
    use std::time::Instant;
    use cw_utils::{PaymentError}; 

    use super::*;
    use cosmwasm_std::{
        from_binary, BlockInfo, CosmosMsg, Env, Addr, MessageInfo,
        QueryResponse, StdError, Uint128, WasmMsg, coins, Api
    };
    use cosmwasm_std::testing::{mock_dependencies, mock_info, mock_env};
    use cosmwasm_storage::{singleton, ReadonlySingleton, Singleton};
    use rand::{RngCore, SeedableRng};
    // use rand_chacha::ChaChaRng;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use sha3::Sha3_256;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let mut msg = InstantiateMsg {
            entropy_beacon_addr: Addr::unchecked("example_contract_address"),
            owner_addr: Addr::unchecked("empty_address"),
            win_amount: Uint128::from(0u128),
            token: Denom::from("USK"), 
            play_amount: Uint128::from(1000000u128),
            fee_amount: Uint128::from(100000u128),
            rule_set: RuleSet {
                zero: Uint128::from(0u128),
                one: Uint128::from(1u128),
                two: Uint128::from(2u128),
                three: Uint128::from(3u128),
                four: Uint128::from(4u128),
                five: Uint128::from(5u128),
                six: Uint128::from(6u128),
            },
        };

        // Ensure the proper contract owner address is set 
        msg.owner_addr = Addr::unchecked("owner_address");
        let verified_owner_address = deps.api.addr_validate(&msg.owner_addr.to_string()).unwrap();
        let info = mock_info("creator", &coins(1000, "USK"));

        // Ensure that the contract has been initialized successfully
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Ensure that the state was stored properly 
        let state = STATE.load(deps.as_ref().storage).unwrap();
        assert_eq!(state.entropy_beacon_addr, "kujira1xwz7fll64nnh4p9q8dyh9xfvqlwfppz4hqdn2uyq2fcmmqtnf5vsugyk7u".to_string());
        assert_eq!(state.owner_addr, "owner".to_string());
        assert_eq!(state.win_amount, Uint128::from(0u128));
        assert_eq!(state.token, Denom::from("USK"));
        assert_eq!(state.play_amount, Uint128::from(1000000u128));
        assert_eq!(state.fee_amount, Uint128::from(100000u128));
        assert_eq!(
            state.rule_set, 
            RuleSet{
                zero: Uint128::from(0u128),
                one: Uint128::from(1u128),
                two: Uint128::from(2u128),
                three: Uint128::from(3u128),
                four: Uint128::from(4u128),
                five: Uint128::from(5u128),
                six: Uint128::from(6u128),
            }
        ); 

        // Ensure that the game index was initialized to zero 
        let idx = IDX.load(deps.as_ref().storage).unwrap();
        assert_eq!(idx, Uint128::from(0u128));

        // Test that win_amount cannot be greater than play_amount 

    }

    #[test]
    fn test_spin() {



        // Set up contract state 
        let entropy_beacon_addr = "entropy_beacon_address".to_string();

        let rule_set = RuleSet { // Winning odds are 1:3:5:10:20:45:45
            zero: Uint128::from(1u128),
            one: Uint128::from(3u128),
            two: Uint128::from(5u128),
            three: Uint128::from(10u128),
            four: Uint128::from(20u128),
            five: Uint128::from(45u128),
            six: Uint128::from(45u128),
        };

        // # PLAYER 1 GAME: Bet 100, Sector 0, Win, Payout 1:1
        let mut gameId = 0u128; 
        let mut player1_deps = mock_dependencies();

        let mut player1_state = State {
            entropy_beacon_addr: Addr::unchecked(entropy_beacon_addr), // The entropy beacon contract address
            owner_addr: Addr::unchecked("owner_address"), // The contract owner
            token: Denom::from("USK"), // The token used for the game
            house_bankroll: Coin {
                denom: "USK".to_string(), 
                amount: Uint128::from(1000u128)}, // Initialize with 1000
            play_amount: Uint128::from(0u128), // The size of the players bet 
            win_amount: Uint128::zero(), // The amount the player wins
            fee_amount: Uint128::zero(), // The amount the player pays in fees
            rule_set: rule_set.clone(), 
        }; 

        
        IDX.save(&mut player1_deps.storage, &Uint128::new(gameId)).unwrap();

        // Create a new game, Player1 bets $USK 100 on number 0
        // Player should win, and recieve a payout of 1:1 (100)
        let player1 = "player1".to_string(); 
        let player1_bet_amount = Uint128::from(100u128);
        let player1_bet_number = 0; 
        let player1_info1 = mock_info(&player1, &coins(100, "USK"));

        // Save the player1 game in the GAME state 
        let player1_game = Game {
            player: Addr::unchecked("player1"),
            bet: player1_bet_number, 
            payout: Uint128::from(0u128),
            result: None, 
            played: false,
            win: None,  
        }; 
        GAME.save(&mut player1_deps.storage, gameId, &player1_game).unwrap();

        player1_state.play_amount = player1_bet_amount.clone();

        // Make sure bet is <= 10% of bank houseroll MAX, convert to float to assert correctly 
        assert!(
            player1_bet_amount.to_string().parse::<f64>().unwrap() 
            <= player1_state.house_bankroll.amount.to_string().parse::<f64>().unwrap() * 0.1);

        STATE.save(&mut player1_deps.storage, &player1_state).unwrap();
        
        // Ensure house bankroll has been updated to $USK 1000
        let state = STATE.load(player1_deps.as_ref().storage).unwrap();
        assert_eq!(Uint128::new(1000), state.house_bankroll.amount); 


        //TODO: Check results in this var in debug mode, works? 
        let exe_beacon_pull = execute_entropy_beacon_pull(
            player1_deps.as_mut(), 
            mock_env(), 
            player1_info1.clone(), 
            player1_bet_amount, 
            player1_bet_number, 
        ).unwrap(
        );

            /*
            pub struct EntropyCallbackMsg {
                /// The entropy that was generated by the network, in a byte array.
                /// The length of the byte array is 64 elements, meaning that there are
                /// 64 total bytes of entropy.
                pub entropy: Vec<u8>,

                /// The original address that submitted the request for entropy. This
                /// should be used to verify that the callback message was generated by
                /// the requester, or by a trusted other contract.
                pub requester: Addr,

                /// The callback message that was specified in the request. The structure
                /// of this Binary is unknown by the Beacon contract, and it is up to your
                /// contract to correctly decode it.
                pub msg: Binary, 
            */

        // Faking game.result created by entropy callback

        let simulated_entropy_result = vec![0u8]; 

        //TODO: Find some other way to test this? Cannot get EntropyCallbackMsg in debug mode?
      /*   let exe_recieve_entropy = execute_recieve_entropy(
            deps.as_mut(), 
            mock_env(), 
            player1_info1.clone(), 
            data, // NEED ENTROPY CB DATA
        ).unwrap(
        );  */
        
        // Faking game.result which is supposed to be generated by execute_recieve_entropy()
        let mut player1_game = GAME.load(player1_deps.as_ref().storage, gameId).unwrap();

        // player1_game.result = Some(get_outcome_from_entropy(&entropy_gen_test)); 
        player1_game.result = Some(simulated_entropy_result.clone());
        GAME.save(&mut player1_deps.storage, gameId, &player1_game).unwrap();

        // Start the game 
        let exe_spin = execute_spin(
            player1_deps.as_mut(), 
            mock_env(), 
            player1_info1.clone(), 
            player1_bet_amount, 
            player1_bet_number, 
        ).unwrap(
        ); 

        // Assert correct attributes from spin  
        assert_eq!(
            exe_spin.attributes, 
            vec![
                ("game".to_string(), "0".to_string()), 
                ("player".to_string(), player1.clone()), 
                ("result".to_string(), "win".to_string()),
                ("payout".to_string(), "100".to_string()),
            ]
        );



     /*    // Assert that the player's bet is saved in the new game 
        let game = GAME.load(deps.as_ref().storage, 0).unwrap();
        assert_eq!(game.bet, player1_bet_number);

        // Try to spin the wheel with insufficient funds 
        let player2 = "player2".to_string();
        let info2 = mock_info(&player2, &coins(0, "USK"));
        let execute_msg_spin = ExecuteMsg::Spin {
            bet_amount: Uint128::zero(), 
        }; 

        let res = execute(
            deps.as_mut(), 
            mock_env(), 
            info2.clone(), 
            execute_msg_spin,
            Uint128::zero(), 
            0, 
        ); 

        match res.unwrap_err() {
            ContractError::InsufficientFunds {  }  => assert!(true), 
            _ => panic!("unexpected error"),
        }


        // Try to spin the wheel with an invalid bet amount 
        let player3 = "player3".to_string();
        let invalid_bet_amount = Uint128::from(10u128); 
        let info3 = mock_info(&player3, &coins(10, "USK"));
        let execute_msg_spin = ExecuteMsg::Spin {
            bet_amount: invalid_bet_amount, 
        };
        let res = execute(
            deps.as_mut(), 
            mock_env(), 
            info3.clone(), 
            execute_msg_spin,
            invalid_bet_amount, 
            0, 
        );
        
        // assert that the error is returned when the player tries to spin the wheel with an invalid bet amount
        assert_eq!(
            res.unwrap_err(), 
            ContractError::Unauthorized {  } // Should be InvalidBet?
        );

        // Try to spin the wheel with an invalid token amount 
        let player4 = "player4".to_string();
        let invalid_token_amount = Uint128::from(100u128);
        let info4 = mock_info(&player4, &coins(100, "INVALID_TOKEN"));
        let execute_msg_spin = ExecuteMsg::Spin {
            bet_amount: invalid_token_amount, 
        };
        let res = execute(
            deps.as_mut(), 
            mock_env(), 
            info4.clone(), 
            execute_msg_spin,
            invalid_token_amount, 
            0, 
        );

        // Assert that the error is returned when the player tries to spin the wheel with an invalid token amount
        assert_eq!(
            res.unwrap_err(), 
            ContractError::InvalidToken {  }
        ); */

    }


}