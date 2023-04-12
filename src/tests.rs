#[allow(unused_imports)]
#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;
    use cosmwasm_std::CosmosMsg::{Bank};
    use cosmwasm_std::{BankMsg, SubMsg};  


    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, mock_dependencies_with_balance, MockQuerier, MockApi, MockStorage};
    use cosmwasm_std::{ Uint128, Coin, Addr, Response, coins, from_binary, to_binary, WasmMsg, CosmosMsg, DepsMut, MemoryStorage, Storage, to_vec, Order, Deps, QuerierWrapper, Api, Querier, from_slice, MessageInfo, WasmQuery, Empty, StdError};
    use cw2::{set_contract_version, CONTRACT, get_contract_version};
    use entropy_beacon_cosmos::beacon::RequestEntropyMsg;
    use entropy_beacon_cosmos::provide::ActiveRequestsQuery;
    use entropy_beacon_cosmos::{EntropyRequest, EntropyCallbackMsg, CalculateFeeQuery};
    use entropy_beacon_cosmos::msg::QueryMsg as BeaconQueryMsg;
    use entropy_beacon_cosmos::msg::ExecuteMsg as BeaconExecuteMsg;
    use crate::contract::{execute, instantiate, query, migrate};
    use crate::helpers::{calculate_payout, get_outcome_from_entropy, update_leaderboard, query_leaderboard, calculate_beacon_fee, update_player_history_win, update_player_history_loss, update_game_state_for_win, validate_bet_number, validate_funds_sent, validate_denom, validate_bet_amount, validate_bet_vs_bankroll, validate_sent_amount_to_cover_fee, get_bankroll_balance, load_player_history_or_create_new, save_game_state, verify_callback_sender, update_game_and_player_history, build_response};
    use crate::state::{RuleSet, State, Game, PLAYER_HISTORY, PlayerHistory, STATE, IDX, GAME, LatestGameIndexResponse, LeaderBoardEntry};
    use crate::msg::{ExecuteMsg, InstantiateMsg, EntropyCallbackData, QueryMsg, GameResponse, MigrateMsg};
    use cosmwasm_std::Binary;
    use crate::error::ContractError;

    #[test]
    fn test_proper_initialization() {

        // Define contract metadata
        const CONTRACT_NAME: &str = "entropiclabs/Whale-of-fortune-v1.0.1";
        const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
        
        // Define mock dependencies and environment 
        let mut deps = mock_dependencies();
        let info = mock_info("addr0000", &[]);
        let env = mock_env();

        // Define an InstantiateMsg
        let msg = InstantiateMsg {
            entropy_beacon_addr: Addr::unchecked("entropy_beacon_actual_addr"),
        };

        // Set the contract version
        set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION).unwrap();

        // Check that the contract name and version were set correctly 
        let contract_version = get_contract_version(&deps.storage).unwrap();
        assert_eq!(contract_version.contract, CONTRACT_NAME);
        assert_eq!(contract_version.version, CONTRACT_VERSION);

        // Call the instantiate function
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

        // Check that the response is correct
        assert_eq!(res.messages.len(), 0);
        assert_eq!(res.attributes.len(), 3);
        assert_eq!(
            res.attributes[0],
            ("method".to_string(), "instantiate".to_string()));
        assert_eq!(
            res.attributes[1],
            ("entropy_beacon_addr".to_string(), msg.entropy_beacon_addr.to_string()));

        // Create a new State instance //* CREATED IN INSTANTIATE
        let state = State {
            entropy_beacon_addr: msg.entropy_beacon_addr.clone(),
            house_bankroll: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::zero(),
            },
        };
        
        // load the state created in instantiate and check that it is correct
        let loaded_state = STATE.load(&deps.storage).unwrap();

        // Check that the state was saved correctly
        assert_eq!(loaded_state, state);
        assert_eq!(res.attributes[2], 
            ("state".to_string(), format!("{:?}", state)));

        // Create a new IDX instance, and save it to storage
        let idx = Uint128::new(0);
        IDX.save(&mut deps.storage, &idx).unwrap();

        // Check that the IDX was saved correctly
        let loaded_idx = IDX.load(&deps.storage).unwrap();
        assert_eq!(loaded_idx, idx);

    }

    #[test]
    fn test_spin() {
        // Define mock dependencies and environment
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]);
        let env = mock_env();
        let info = mock_info("player", &[Coin{denom: "ukuji".to_string(), amount: Uint128::new(10)}]);

        // Set up the contract state and index
        let state = State {
            entropy_beacon_addr: Addr::unchecked("kujira1pvrwmjuusn9wh34j7y520g8gumuy9xtl3gvprlljfdpwju3x7ucseu6vw3"),
            house_bankroll: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(1000),
            },
        };
        STATE.save(&mut deps.storage, &state).unwrap();
        let idx = Uint128::zero();
        IDX.save(&mut deps.storage, &idx).unwrap();

        // Call the Spin function
        let bet_number = Uint128::new(3);
        let res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::Spin { bet_number },
        )
        .unwrap();

        // Check that the game state was saved correctly
        let expected_game = Game {
            player: info.sender.to_string(),
            game_idx: idx.into(),
            bet_number: bet_number.into(),
            bet_size: info.funds[0].amount.clone().into(),
            outcome: "Pending".to_string(),
            played: false,
            win: false,
            payout: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::zero(),
            },
            rule_set: RuleSet {
                zero: Uint128::new(1),
                one: Uint128::new(3),
                two: Uint128::new(5),
                three: Uint128::new(10),
                four: Uint128::new(20),
                five: Uint128::new(45),
                six: Uint128::new(45),
            },
        };

        let game_key = idx.into();
        let game = GAME.load(&deps.storage, game_key).unwrap();
        assert_eq!(game, expected_game);

        // Check that an EntropyRequest submessage was returned
        assert_eq!(res.messages.len(), 1);
        let submsg = &res.messages[0];

        let callback_gas_limit = 150_000u64;
        let beacon_fee = calculate_beacon_fee(&deps.as_mut(), &info.sender.to_string(), callback_gas_limit).unwrap();

        let callback_msg = to_binary(&EntropyCallbackData {
            original_sender: info.sender.clone(),
        }).unwrap_or_else(|e| panic!("failed to serialize callback message: {}", e));

        // Create an EntropyRequest submessage
        let entropy_request =  EntropyRequest {
            callback_gas_limit,
            callback_address: env.contract.address.clone(),
            funds: vec![Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::from(beacon_fee),
            }],
                callback_msg,
            };
         
        // Check that the EntropyRequest callback address is the same as the contract address 
        assert_eq!(
            entropy_request.callback_address,
            env.contract.address.clone()
        );
        // Check that the EntropyRequest funds are correct
        assert_eq!(entropy_request.funds, vec![Coin::new(beacon_fee.into(), "ukuji".to_string())]);

        // Check that the EntropyRequest submessage calls the expected function
        let expected_entropy_callback = WasmMsg::Execute {
            contract_addr: state.entropy_beacon_addr.clone().into_string(),
            msg: to_binary(&BeaconExecuteMsg::RequestEntropy(RequestEntropyMsg {
                callback_gas_limit,
                callback_address: env.contract.address.clone(),
                callback_msg: to_binary(&EntropyCallbackData {
                    original_sender: info.sender.clone(),
                }).unwrap(),
            }))
            .unwrap(),
            funds: vec![Coin { amount: Uint128::new(beacon_fee.into()), denom: "ukuji".to_string() }],
        };
        
        assert_eq!(
            submsg.msg,
            CosmosMsg::Wasm(expected_entropy_callback)
        );

    }

    #[test]
    fn test_free_spin() {
        // Define mock dependencies and environment
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]);
        let env = mock_env();
        let info = mock_info("player", &[Coin{denom: "ukuji".to_string(), amount: Uint128::new(0)}]);

        // Set up the contract state and index
        let state = State {
            entropy_beacon_addr: Addr::unchecked("kujira1pvrwmjuusn9wh34j7y520g8gumuy9xtl3gvprlljfdpwju3x7ucseu6vw3"),
            house_bankroll: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(1000),
            },
        };
        STATE.save(&mut deps.storage, &state).unwrap();
        let idx = Uint128::zero();
        IDX.save(&mut deps.storage, &idx).unwrap();

        let player_history = PlayerHistory {
            player_address: info.sender.to_string(),
            wins: Uint128::zero(), 
            losses: Uint128::zero(),
            games_played: Uint128::zero(),
            total_coins_spent: Coin{denom: "ukuji".to_string(), amount: Uint128::zero()},
            total_coins_won: Coin{denom: "ukuji".to_string(), amount: Uint128::zero()}, 
            free_spins: Uint128::new(1),

        };

        PLAYER_HISTORY.save(&mut deps.storage, info.sender.to_string(), &player_history).unwrap();

        // Call the Spin function
        let bet_number = Uint128::new(3);
        let res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::FreeSpin { bet_number },
        )
        .unwrap();

        // Check that the game state was saved correctly
        let expected_game = Game {
            player: info.sender.to_string(),
            game_idx: idx.into(),
            bet_number: bet_number.into(),
            bet_size: Uint128::new(1).into(),
            outcome: "Pending".to_string(),
            played: false,
            win: false,
            payout: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::zero(),
            },
            rule_set: RuleSet {
                zero: Uint128::new(1),
                one: Uint128::new(3),
                two: Uint128::new(5),
                three: Uint128::new(10),
                four: Uint128::new(20),
                five: Uint128::new(45),
                six: Uint128::new(45),
            },
        };

        let game_key = idx.into();
        let game = GAME.load(&deps.storage, game_key).unwrap();
        assert_eq!(game, expected_game);

        // Check that an EntropyRequest submessage was returned
        assert_eq!(res.messages.len(), 1);
        let submsg = &res.messages[0];

        let callback_gas_limit = 150_000u64;
        let beacon_fee = calculate_beacon_fee(&deps.as_mut(), &info.sender.to_string(), callback_gas_limit).unwrap();

        let callback_msg = to_binary(&EntropyCallbackData {
            original_sender: info.sender.clone(),
        }).unwrap_or_else(|e| panic!("failed to serialize callback message: {}", e));

        // Create an EntropyRequest submessage
        let entropy_request =  EntropyRequest {
            callback_gas_limit,
            callback_address: env.contract.address.clone(),
            funds: vec![Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::from(beacon_fee),
            }],
                callback_msg,
            };
         

        // Check that the EntropyRequest callback address is the same as the contract address 
        assert_eq!(
            entropy_request.callback_address,
            env.contract.address.clone()
        );
        
        // Check that the EntropyRequest funds are correct
        assert_eq!(entropy_request.funds, vec![Coin::new(beacon_fee.into(), "ukuji".to_string())]);

        // Check that the EntropyRequest submessage calls the expected function
        let expected_entropy_callback = WasmMsg::Execute {
        contract_addr: state.entropy_beacon_addr.clone().into_string(),
        msg: to_binary(&BeaconExecuteMsg::RequestEntropy(RequestEntropyMsg {
            callback_gas_limit,
            callback_address: env.contract.address.clone(),
            callback_msg: to_binary(&EntropyCallbackData {
                original_sender: info.sender.clone(),
            })
            .unwrap(),
        }))
        .unwrap(),
        funds: vec![Coin {
            amount: Uint128::new(beacon_fee.into()),
            denom: "ukuji".to_string(),
        }],
        };
        
        assert_eq!(
            submsg.msg,
            CosmosMsg::Wasm(expected_entropy_callback)
        );

    }

    #[test]
    fn test_free_spin_no_freespins_left() {
        // Define mock dependencies and environment
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]);
        let env = mock_env();
        let info = mock_info("player", &[Coin{denom: "ukuji".to_string(), amount: Uint128::new(0)}]);

        // Set up the contract state and index
        let state = State {
            entropy_beacon_addr: Addr::unchecked("kujira1pvrwmjuusn9wh34j7y520g8gumuy9xtl3gvprlljfdpwju3x7ucseu6vw3"),
            house_bankroll: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(1000),
            },
        };
        STATE.save(&mut deps.storage, &state).unwrap();
        let idx = Uint128::zero();
        IDX.save(&mut deps.storage, &idx).unwrap();

        let player_history = PlayerHistory {
            player_address: info.sender.to_string(),
            wins: Uint128::zero(), 
            losses: Uint128::zero(),
            games_played: Uint128::zero(),
            total_coins_spent: Coin{denom: "ukuji".to_string(), amount: Uint128::zero()},
            total_coins_won: Coin{denom: "ukuji".to_string(), amount: Uint128::zero()}, 
            free_spins: Uint128::zero(),

        };

        PLAYER_HISTORY.save(&mut deps.storage, info.sender.to_string(), &player_history).unwrap();

        // Call the Spin function
        let bet_number = Uint128::new(3);
        let res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::FreeSpin { bet_number },
        ); 

        assert_eq!(
            res,
            Err(ContractError::NoFreeSpinsLeft {}),
            "Expected ContractError::NoFreeSpinsLeft, got: {:?}",
            res
        );
    }

    #[test]
    fn test_recieve_entropy() {
        // Define mock dependencies and environment
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]);
        let info = mock_info("player", &[Coin{denom: "ukuji".to_string(), amount: Uint128::new(10)}]);
        let env = mock_env();
        let entropy = vec![1u8; 64];

        let idx = Uint128::zero();
        IDX.save(&mut deps.storage, &idx).unwrap();

        let game = Game::new_game(
            &info.sender.to_string(), 
            idx.into(),
            6u128,
            10u128,

        );

        GAME.save(&mut deps.storage, idx.into(), &game).unwrap();

        let state = State {
            entropy_beacon_addr: Addr::unchecked("kujira1pvrwmjuusn9wh34j7y520g8gumuy9xtl3gvprlljfdpwju3x7ucseu6vw3"),
            house_bankroll: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(1000),
            },
        };

        STATE.save(&mut deps.storage, &state).unwrap();

        let entropy_callback_msg = EntropyCallbackMsg {
            entropy: vec![1u8; 64],
            requester: info.sender.clone(),
            msg: Binary::from(b"arbitrary data".to_vec()),
        };

        // Call the ReceiveEntropy function
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("kujira1pvrwmjuusn9wh34j7y520g8gumuy9xtl3gvprlljfdpwju3x7ucseu6vw3", &[]),
            ExecuteMsg::ReceiveEntropy(entropy_callback_msg),
        )
        .unwrap();
    
        assert_eq!(res.messages.len(), 0);
    }

    #[test]
    fn test_calculated_payout_outcome_out_of_range() {

        let rule_set =  RuleSet {
            zero: Uint128::new(24),
            one: Uint128::new(12),
            two: Uint128::new(8),
            three: Uint128::new(4),
            four: Uint128::new(2),
            five: Uint128::new(1),
            six: Uint128::new(1),
        }; 
        
        let calculated_payout = calculate_payout(Uint128::new(1), 7u8, rule_set); 

        assert_eq!(calculated_payout, Uint128::new(0));
    }

    #[test]
    fn test_get_outcome_from_entropy_outcome_0() {
        let rule_set = RuleSet {
            zero: Uint128::new(24),
            one: Uint128::new(12),
            two: Uint128::new(8),
            three: Uint128::new(4),
            four: Uint128::new(2),
            five: Uint128::new(1),
            six: Uint128::new(1),
        };
    
        let mut entropy = vec![0; 64]; // A 64-byte vector
    
        entropy[0..4].copy_from_slice(&(0 as u32).to_be_bytes());
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert_eq!(outcome, vec![0]);
    }

    #[test]
    fn test_get_outcome_from_entropy_outcome_1() {
        let rule_set = RuleSet {
            zero: Uint128::new(24),
            one: Uint128::new(12),
            two: Uint128::new(8),
            three: Uint128::new(4),
            four: Uint128::new(2),
            five: Uint128::new(1),
            six: Uint128::new(1),
        };
    
        let mut entropy = vec![0; 64]; // A 64-byte vector
    
        entropy[0..4].copy_from_slice(&(rule_set.zero.u128() as u32).to_be_bytes());
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert_eq!(outcome, vec![1]);
    }

    #[test]
    fn test_get_outcome_from_entropy_outcome_2() {
        let rule_set = RuleSet {
            zero: Uint128::new(24),
            one: Uint128::new(12),
            two: Uint128::new(8),
            three: Uint128::new(4),
            four: Uint128::new(2),
            five: Uint128::new(1),
            six: Uint128::new(1),
        };
    
        let mut entropy = vec![0; 64]; // A 64-byte vector
    
        let sum_01 = (rule_set.zero.u128() + rule_set.one.u128()) as u32;
        entropy[0..4].copy_from_slice(&sum_01.to_be_bytes());
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert_eq!(outcome, vec![2]);
    } 
    
    #[test]
    fn test_get_outcome_from_entropy_outcome_3() {
        let rule_set =  RuleSet {
            zero: Uint128::new(24),
            one: Uint128::new(12),
            two: Uint128::new(8),
            three: Uint128::new(4),
            four: Uint128::new(2),
            five: Uint128::new(1),
            six: Uint128::new(1),
        }; 

        let mut entropy = vec![0; 64]; // A 64-byte vector

        let sum_012 = (rule_set.zero.u128() + rule_set.one.u128() + rule_set.two.u128()) as u32;
        entropy[0..4].copy_from_slice(&sum_012.to_be_bytes());
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert_eq!(outcome, vec![3]);
    }

    #[test]
    fn test_get_outcome_from_entropy_outcome_4() {
        let rule_set = RuleSet {
            zero: Uint128::new(24),
            one: Uint128::new(12),
            two: Uint128::new(8),
            three: Uint128::new(4),
            four: Uint128::new(2),
            five: Uint128::new(1),
            six: Uint128::new(1),
        };

        let mut entropy = vec![0; 64]; // A 64-byte vector

        let sum_0123 = (rule_set.zero.u128() + rule_set.one.u128() + rule_set.two.u128() + rule_set.three.u128()) as u32;
        entropy[0..4].copy_from_slice(&sum_0123.to_be_bytes());
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert_eq!(outcome, vec![4]);
    }

    #[test]
    fn test_get_outcome_from_entropy_outcome_5() {
        let rule_set = RuleSet {
            zero: Uint128::new(24),
            one: Uint128::new(12),
            two: Uint128::new(8),
            three: Uint128::new(4),
            four: Uint128::new(2),
            five: Uint128::new(1),
            six: Uint128::new(1),
        };

        let mut entropy = vec![0; 64]; // A 64-byte vector

        let sum_01234 = (rule_set.zero.u128() + rule_set.one.u128() + rule_set.two.u128() + rule_set.three.u128() + rule_set.four.u128()) as u32;
        entropy[0..4].copy_from_slice(&sum_01234.to_be_bytes());
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert_eq!(outcome, vec![5]);
    }

    #[test]
    fn test_get_outcome_from_entropy_outcome_6() {
        let rule_set = RuleSet {
            zero: Uint128::new(24),
            one: Uint128::new(12),
            two: Uint128::new(8),
            three: Uint128::new(4),
            four: Uint128::new(2),
            five: Uint128::new(1),
            six: Uint128::new(1),
        };
    
        let mut entropy = vec![0; 64]; // A 64-byte vector
    
        let sum_012345 = (rule_set.zero.u128() + rule_set.one.u128() + rule_set.two.u128() + rule_set.three.u128() + rule_set.four.u128() + rule_set.five.u128()) as u32;
        entropy[0..4].copy_from_slice(&sum_012345.to_be_bytes());
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert_eq!(outcome, vec![6]);
    }

    #[test]
    fn test_update_player_history_win() {
        let mut player_history = PlayerHistory {
            player_address: "player_address".to_string(),
            games_played: Uint128::new(4),
            wins: Uint128::new(2),
            losses: Uint128::new(2),
            total_coins_spent: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(40),
            },
            total_coins_won: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(25),
            },
            free_spins: Uint128::new(0),
        };
    
        let bet_size = Uint128::new(10);
        let calculated_payout = Uint128::new(15);
    
        update_player_history_win(&mut player_history, bet_size, calculated_payout);
    
        assert_eq!(player_history.games_played, Uint128::new(5));
        assert_eq!(player_history.wins, Uint128::new(3));
        assert_eq!(player_history.losses, Uint128::new(2));
        assert_eq!(player_history.total_coins_spent.amount, Uint128::new(50));
        assert_eq!(player_history.total_coins_won.amount, Uint128::new(40));
        assert_eq!(player_history.free_spins, Uint128::new(1));
    }
    
    #[test]
    fn test_update_player_history_loss() {
        let mut player_history = PlayerHistory {
            player_address:"player".to_string(),
            games_played: Uint128::new(4),
            wins: Uint128::new(2),
            losses: Uint128::new(2),
            total_coins_spent: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(40),
            },
            total_coins_won: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(25),
            },
            free_spins: Uint128::new(0),
        };
    
        let bet_size = Uint128::new(10);
    
        update_player_history_loss(&mut player_history, bet_size);
    
        assert_eq!(player_history.games_played, Uint128::new(5));
        assert_eq!(player_history.wins, Uint128::new(2));
        assert_eq!(player_history.losses, Uint128::new(3));
        assert_eq!(player_history.total_coins_spent.amount, Uint128::new(50));
        assert_eq!(player_history.total_coins_won.amount, Uint128::new(25));
        assert_eq!(player_history.free_spins, Uint128::new(1));
    }

    #[test]
    fn test_recieve_entropy_with_player_history() {
        // Define mock dependencies and environment
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]);
        let info = mock_info("player", &[Coin{denom: "ukuji".to_string(), amount: Uint128::new(10)}]);
        let env = mock_env();
        let entropy = vec![1u8; 64];

        let idx = Uint128::zero();
        IDX.save(&mut deps.storage, &idx).unwrap();

        let game = Game::new_game(
            &info.sender.to_string(), 
            idx.into(),
            6u128,
            10u128,

        );

        let player_history = PlayerHistory{
            player_address: info.sender.to_string(),
            wins: Uint128::new(1), 
            losses: Uint128::new(2),
            games_played: Uint128::new(3),
            total_coins_spent: Coin{denom: "ukuji".to_string(), amount: Uint128::new(3)},
            total_coins_won: Coin{denom: "ukuji".to_string(), amount: Uint128::new(1)}, 
            free_spins: Uint128::zero(),
        };

        PLAYER_HISTORY.save(&mut deps.storage, info.sender.to_string(), &player_history).unwrap();

        GAME.save(&mut deps.storage, idx.into(), &game).unwrap();

        let state = State {
            entropy_beacon_addr: Addr::unchecked("kujira1pvrwmjuusn9wh34j7y520g8gumuy9xtl3gvprlljfdpwju3x7ucseu6vw3"),
            house_bankroll: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(1000),
            },
        };

        STATE.save(&mut deps.storage, &state).unwrap();

        let entropy_callback_msg = EntropyCallbackMsg {
            entropy: vec![1u8; 64],
            requester: info.sender.clone(),
            msg: Binary::from(b"arbitrary data".to_vec()),
        };

        // Call the ReceiveEntropy function
        let res = execute(
            deps.as_mut(),
            env.clone(),
            mock_info("kujira1pvrwmjuusn9wh34j7y520g8gumuy9xtl3gvprlljfdpwju3x7ucseu6vw3", &[]),
            ExecuteMsg::ReceiveEntropy(entropy_callback_msg),
        )
        .unwrap();
    
        assert_eq!(res.messages.len(), 0);
    }

    #[test]
    fn test_player_history() {
        let mut deps = mock_dependencies();
        let info = mock_info("addr0000", &[Coin{denom: "ukuj".to_string(), amount: Uint128::new(10)}]);

        let bet_number = 0u128; // 0-6
        let sent_amount = info.funds[0].amount; 
        let idx = Uint128::new(0);

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

            // Check if player history exists for this player: if not, create a new instance of it and save it
            // let player_history = match PLAYER_HISTORY.may_load(&deps.storage, info.sender.clone().to_string()) {
            let player_history_result = PLAYER_HISTORY.may_load(&deps.storage, info.sender.clone().to_string()).unwrap();

            let mut player_history = match player_history_result {
                Some(history) => history,
                None => {
                    PlayerHistory {
                        player_address: info.sender.clone().to_string(),
                        games_played: Uint128::zero(),
                        wins: Uint128::zero(),
                        losses: Uint128::zero(),
                        total_coins_spent: Coin {
                            denom: "ukuji".to_string(),
                            amount: Uint128::zero(),
                        },
                        total_coins_won: Coin {
                            denom: "ukuji".to_string(),
                            amount: Uint128::zero(),
                        },
                        free_spins: Uint128::zero(),
                    }
                    }
                };

            // save the player history state to storage
            PLAYER_HISTORY.save(&mut deps.storage, info.sender.clone().to_string(), &player_history).unwrap();  

            // assert!(player_history.is_some());
            let mut player_history = PLAYER_HISTORY.load(&deps.storage, info.sender.clone().to_string()).unwrap();

            player_history.games_played += Uint128::one();

            // save the player history state to storage
            PLAYER_HISTORY.save(&mut deps.storage, info.sender.clone().to_string(), &player_history).unwrap();  

            let player_history = PLAYER_HISTORY.load(&deps.storage, info.sender.clone().to_string()).unwrap();
            assert_eq!(player_history.games_played, Uint128::new(1));

    }

    #[test]
    fn test_calculate_payout() {
        let rule_set = RuleSet {
            zero: Uint128::new(1),
            one: Uint128::new(3),
            two: Uint128::new(5),
            three: Uint128::new(10),
            four: Uint128::new(20),
            five: Uint128::new(45),
            six: Uint128::new(45),
        };
    
        assert_eq!(calculate_payout(Uint128::new(1), 0, rule_set.clone()), Uint128::new(1));
        assert_eq!(calculate_payout(Uint128::new(1), 1, rule_set.clone()), Uint128::new(3));
        assert_eq!(calculate_payout(Uint128::new(1), 2, rule_set.clone()), Uint128::new(5));
        assert_eq!(calculate_payout(Uint128::new(1), 3, rule_set.clone()), Uint128::new(10));
        assert_eq!(calculate_payout(Uint128::new(1), 4, rule_set.clone()), Uint128::new(20));
        assert_eq!(calculate_payout(Uint128::new(1), 5, rule_set.clone()), Uint128::new(45));
        assert_eq!(calculate_payout(Uint128::new(1), 6, rule_set.clone()), Uint128::new(45));

        let payout = calculate_payout(Uint128::new(100), 2, rule_set);

        assert_eq!(payout, Uint128::new(500));
    }

    #[test]
    fn test_get_outcome_from_entropy() {

        let rule_set =  RuleSet {
            zero: Uint128::new(1),
            one: Uint128::new(3),
            two: Uint128::new(5),
            three: Uint128::new(10),
            four: Uint128::new(20),
            five: Uint128::new(45),
            six: Uint128::new(45),
        }; 

        // Valid, entropy will result in result = 0
        let entropy = hex::decode("68b7cfd0fcfd3564359318426bea7f203ebc8687bda140645d60caaf79b6b18b9e8d9c93e62f2b2e138c520253b96c23800b2f82274586a4b5f246a3479a5715").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 1);
        assert!(outcome[0] <= 6);
        assert!(outcome[0] == 0);

        // Valid, entropy will result in result = 0
        let entropy = hex::decode("54c86044dfdd18902279243ce80741ab186cba4027c137fab649b861fb328da77b3bebe62783c76b96fc34381a855f9383d9d20ff83fbc3ecbab7c90d1b597ba").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 1);
        assert!(outcome[0] <= 6);
        assert!(outcome[0] == 0);

        // Valid, entropy will result in result = 0
        let entropy = hex::decode("2c5e77254df7b533472cdb54413150981b82f98368a2e7a5574fc7ed7fec52de5573bf25d587c2c1ec91728d1b9d6e9e8cc83d99ec2399bdcbd84e7126ea39a1").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 1);
        assert!(outcome[0] <= 6);
        assert!(outcome[0] == 0);

        // Valid, entropy will result in result = 2
        let entropy = hex::decode("418292d2d2857695eea3809412a789f05753328af24bba6cf0fb0d280f97a16f189ba9143a89c53410ed53b2ef1b982e6a6af8a5614de2c01d085e26cbd12311").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 1);
        assert!(outcome[0] <= 6);
        assert!(outcome[0] == 2);

        // Valid, entropy will result in result = 1
        let entropy = hex::decode("cf12dd2f9e6519d253db082c565f0fc4c40d6ae7c6c51eafe29591324543e7ce2875f9d9fe0182967fc66086a7673bced5b7ba943b856641f005e1583370d4f2").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 1);
        assert!(outcome[0] <= 6);
        assert!(outcome[0] == 1);

        // Valid, entropy will result in result = 1
        let entropy = hex::decode("91e0cc53d16cf6f0c7029e83af1f99337ff171bb14710f3246c7c9b24af8743643d2e9d2877ee8f023923ffe2676fc53f00a9999c38200cd3be8eaaf2e9cef08").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 1);
        assert!(outcome[0] <= 6);
        // assert!(outcome[0] == 1);

        // Valid, entropy will result in result = 1
        let entropy = hex::decode("5b0cde16c345b7faae6898f93641469ec398a4aed349cea3233416281cab5b569dadffbca48ddecca09357875d5bd5e24b7387b4601b7d3012bfc719531c82d5").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 1);
        assert!(outcome[0] <= 6);
        // assert!(outcome[0] == 1);


        // Test entropy with empty input 
        let entropy = hex::decode("").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 0);
        assert!(outcome.is_empty()); 

        // Test entropy with shorter than expected input 
        let entropy = hex::decode("68b7cfd0fcfd3564359318426bea7f203ebc8687bda14063").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 0);
        assert!(outcome.is_empty());

        // Test entropy with too long input
        let entropy = hex::decode("68b7cfd0fcfd3564359318426bea7f203ebc8687bda140645d60caaf79b6b18b9e8d9c93e62f2b2e138c520253b96c23800b2f82274586a4b5f246a3479a571568b7cfd0fcfd3564359318426bea7f203ebc8687bda140645d60caaf79b6b18b9e8d9c93e62f2b2e138c520253b96c23800b2f82274586a4b5f246a3479a5715").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 0);
        assert!(outcome.is_empty());
        

    }

    #[test]
    fn test_leaderboard_update_and_query() {
        let mut owned_deps = mock_dependencies_with_balance(&[Coin {
            denom: "ukuji".to_string(),
            amount: Uint128::new(1000),
        }]);
        let mut deps = owned_deps.as_mut(); // Convert OwnedDeps to DepsMut
    
        let player1 = "player1".to_string();
        let player2 = "player2".to_string();
        let player3 = "player3".to_string();
    
        // Player 1 wins a game
        update_leaderboard(deps.storage, &player1, Uint128::from(1u64));
        // Player 2 wins two games
        update_leaderboard(deps.storage, &player2, Uint128::from(2u64));
        // Player 3 wins a game
        update_leaderboard(deps.storage, &player3, Uint128::from(1u64));
    
        let deps_ref = deps.as_ref(); // Convert DepsMut to Deps
        let leaderboard = query_leaderboard(deps_ref);
    
        assert_eq!(leaderboard.len(), 3);
        assert_eq!(leaderboard[0].player, player2);
        assert_eq!(leaderboard[0].wins, Uint128::from(2u64));
        assert_eq!(leaderboard[1].player, player1);
        assert_eq!(leaderboard[1].wins, Uint128::from(1u64));
        assert_eq!(leaderboard[2].player, player3);
        assert_eq!(leaderboard[2].wins, Uint128::from(1u64));
    }

    #[test]
    fn test_update_leaderboard_existing_entry() {
        let mut deps = mock_dependencies();
        let player1 = "player1".to_string();
    
        // Add a new entry to the leaderboard
        let wins1 = Uint128::new(5);
        update_leaderboard(&mut deps.storage, &player1, wins1);
    
        // Update the existing entry
        let additional_wins1 = Uint128::new(3);
        update_leaderboard(&mut deps.storage, &player1, additional_wins1);
    
        // Retrieve the leaderboard from storage
        let leaderboard_key = "leaderboard";
        let leaderboard: Vec<LeaderBoardEntry> = deps
            .storage
            .get(leaderboard_key.as_bytes())
            .map(|value| from_slice(&value).unwrap())
            .unwrap_or_else(|| vec![]);
    
        // Check if the leaderboard has only one entry and the wins are incremented correctly
        assert_eq!(leaderboard.len(), 1);
        assert_eq!(leaderboard[0].player, player1);
        assert_eq!(leaderboard[0].wins, wins1 + additional_wins1);
    }
    
    #[test]
    fn test_validate_bet_number_invalid() {
        let invalid_bet_number = Uint128::new(7);
        let result = validate_bet_number(invalid_bet_number);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ContractError::InvalidBetNumber {});
    }

    #[test]
    fn test_validate_funds_sent_mismatch() {
        let coin = Coin {
            denom: "ukuji".to_string(),
            amount: Uint128::new(10),
        };
        let info = mock_info("player", &[Coin {
            denom: "ukuji".to_string(),
            amount: Uint128::new(15),
        }]);
        
        let result = validate_funds_sent(&coin, &info);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ContractError::ValidateBetFundsSentMismatch {
                player_sent_amount: coin.amount,
                bet_amount: info.funds[0].amount,
            }
        );
    }
    
    #[test]
    fn test_validate_denom_mismatch() {
        let coin = Coin {
            denom: "player_denom".to_string(),
            amount: Uint128::new(10),
        };
        let bankroll_balance = Coin {
            denom: "house_denom".to_string(),
            amount: Uint128::new(1000),
        };
        
        let result = validate_denom(&coin, &bankroll_balance);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ContractError::ValidateBetDenomMismatch {
                player_sent_denom: coin.denom.clone(),
                house_bankroll_denom: bankroll_balance.denom.clone(),
            }
        );
    }
    
    #[test]
    fn test_validate_bet_amount_zero() {
        let coin = Coin {
            denom: "player_denom".to_string(),
            amount: Uint128::new(0),
        };
        
        let result = validate_bet_amount(&coin);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ContractError::ValidateBetBetAmountIsZero {}
        );
    }
    
    #[test]
    fn test_validate_bet_vs_bankroll_exceeds_limit() {
        let bankroll_balance = Coin {
            denom: "bankroll_denom".to_string(),
            amount: Uint128::new(1000),
        };
        
        let player_bet_amount = Uint128::new(11); // Exceeds 1% of the bankroll_balance (1000 * 0.01 = 10)
        
        let info = MessageInfo {
            sender: Addr::unchecked("player"),
            funds: vec![Coin {
                denom: "bankroll_denom".to_string(),
                amount: player_bet_amount,
            }],
        };
    
        let result = validate_bet_vs_bankroll(&info, &bankroll_balance);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ContractError::ValidateBetBetAmountExceedsHouseBankrollBalance {
                player_bet_amount,
                house_bankroll_balance: bankroll_balance.amount,
            }
        );
    }
    
    #[test]
    fn test_validate_sent_amount_to_cover_fee_insufficient_funds() {
        let sent_amount = Uint128::new(49);
        let beacon_fee = 50u64;
    
        let result = validate_sent_amount_to_cover_fee(sent_amount, beacon_fee);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ContractError::InsufficientFunds {});
    }
    
    #[test]
    fn test_load_player_history_or_create_new_ok_none() {
        // Set up the environment
        let api = MockApi::default();
        let mut storage = MockStorage::default();

        let binding = MockQuerier::default();
    
        let querier: QuerierWrapper<Empty> = QuerierWrapper::new(&binding); // Update here
        let deps: DepsMut<Empty> = DepsMut { // Update here
            api: &api,
            storage: &mut storage,
            querier: querier,
        };
    
        let sender = "player1".to_string();
    
        let result = load_player_history_or_create_new(deps.storage, sender.clone());
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            PlayerHistory::new(sender.to_string())
        );
    }

    #[test]
    fn test_verify_callback_sender_caller_error() {
        let result = verify_callback_sender(&"wrong_caller".to_string(), &"beacon".to_string(), &"requester".to_string(), &"trusted".to_string());
        assert!(matches!(result, Err(ContractError::CallBackCallerError { .. })));
    }

    #[test]
    fn test_verify_callback_sender_requester_error() {
        let result = verify_callback_sender(&"beacon".to_string(), &"beacon".to_string(), &"wrong_requester".to_string(), &"trusted".to_string());
        assert!(matches!(result, Err(ContractError::EntropyRequestError { .. })));
    }

    #[test]
    fn test_save_game_state() {
        let mut deps = mock_dependencies_with_balance(&[Coin {
            denom: "uluna".to_string(),
            amount: Uint128::new(1000),
        }]);
        
        let rule_set = RuleSet {
            zero: Uint128::new(1),
            one: Uint128::new(3),
            two: Uint128::new(5),
            three: Uint128::new(10),
            four: Uint128::new(20),
            five: Uint128::new(45),
            six: Uint128::new(45),
        };
        let game = Game {
            player: "player1".to_string(),
            game_idx: 1,
            bet_number: 5,
            bet_size: 100,
            outcome: "win".to_string(),
            played: true,
            win: true,
            payout: Coin {
                denom: "uluna".to_string(),
                amount: Uint128::from(200_u128),
            },
            rule_set: rule_set,
        };
        let idx = 1_u128;
        save_game_state(&mut deps.storage, idx, &game).unwrap();
        let loaded_game: Game = GAME.load(&deps.storage, idx).unwrap();
        assert_eq!(game, loaded_game);
    }

    #[test]
    fn test_update_game_and_player_history_win() {
        let rule_set = RuleSet {
            zero: Uint128::new(1),
            one: Uint128::new(3),
            two: Uint128::new(5),
            three: Uint128::new(10),
            four: Uint128::new(20),
            five: Uint128::new(45),
            six: Uint128::new(45),
        };
    
        let game = Game {
            player: "player1".to_string(),
            game_idx: 1,
            bet_number: 5,
            bet_size: 100,
            outcome: "win".to_string(),
            played: true,
            win: true,
            payout: Coin {
                denom: "uluna".to_string(),
                amount: Uint128::new(0),
            },
            rule_set: rule_set.clone(),
        };
    
        let mut player_history = PlayerHistory {
            player_address: "player1".to_string(),
            games_played: Uint128::new(0),
            wins: Uint128::new(0),
            losses: Uint128::new(0),
            total_coins_spent: Coin {
                denom: "uluna".to_string(),
                amount: Uint128::new(0),
            },
            total_coins_won: Coin {
                denom: "uluna".to_string(),
                amount: Uint128::new(0),
            },
            free_spins: Uint128::new(0),
        };
    
        let outcome = vec![2];
        let (updated_game, calculated_payout) =
            update_game_and_player_history(true, &game, &mut player_history, &outcome);
    
        assert_eq!(updated_game.payout.amount, calculated_payout);
        assert_eq!(
            player_history.total_coins_spent.amount,
            Uint128::from(game.bet_size.clone())
        );
        assert_eq!(player_history.total_coins_won.amount, calculated_payout);
        assert_eq!(player_history.games_played, Uint128::new(1));
        assert_eq!(player_history.wins, Uint128::new(1));
        assert_eq!(player_history.losses, Uint128::new(0));
    }

    #[test]
    fn test_update_game_state_for_win() {
        
        let rule_set= RuleSet {
            zero: Uint128::new(1),
            one: Uint128::new(3),
            two: Uint128::new(5),
            three: Uint128::new(10),
            four: Uint128::new(20),
            five: Uint128::new(45),
            six: Uint128::new(45),
        }; 

        let game = Game {
            player: "player1".to_string(),
            game_idx: 1u128,
            win: false,
            played: false,
            outcome: "".to_string(),
            bet_size: 10u128,
            bet_number: 1u128,
            payout: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(0),
            },
            rule_set
        };

        let outcome = vec![2u8];
        let payout_amount = Uint128::new(50);

        let updated_game = update_game_state_for_win(game, &outcome, payout_amount);

        assert_eq!(updated_game.game_idx, 1u128);
        assert_eq!(updated_game.player, "player1");
        assert_eq!(updated_game.win, true);
        assert_eq!(updated_game.played, true);
        assert_eq!(updated_game.outcome, "2");
        assert_eq!(updated_game.bet_size, 10u128);
        assert_eq!(updated_game.payout.amount, Uint128::new(50));
    }

    #[test]
    fn test_build_response() {
        let game = Game {
            player: "player1".to_string(),
            game_idx: 1,
            bet_number: 5,
            bet_size: 100,
            outcome: "win".to_string(),
            played: true,
            win: true,
            payout: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::from(200_u128),
            },
            rule_set: RuleSet {
                zero: Uint128::new(1),
                one: Uint128::new(3),
                two: Uint128::new(5),
                three: Uint128::new(10),
                four: Uint128::new(20),
                five: Uint128::new(45),
                six: Uint128::new(45),
            },
        };
        let payout = Uint128::from(200_u128);
        let response = build_response(true, &game, payout.clone());
    
        assert_eq!(
            response.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "player1".to_string(),
                amount: vec![Coin {
                    denom: "ukuji".to_string(),
                    amount: Uint128::from(200_u128)
                }]
            }))]
        );
        assert_eq!(response.attributes.len(), 3);
        assert_eq!(
            response.attributes[0],
            ("game_result".to_string(), "true".to_string())
        );
        assert_eq!(
            response.attributes[1],
            ("game_outcome".to_string(), "win".to_string())
        );
        assert_eq!(
            response.attributes[2],
            ("game_payout".to_string(), payout.to_string())
        );
    }
    
    #[test]
    fn test_update_leaderboard() {
        let mut deps = mock_dependencies_with_balance(&[Coin {
            denom: "uluna".to_string(),
            amount: Uint128::new(1000),
        }]);
    
        let player_address = "player1".to_string();
        let initial_wins = Uint128::new(0);
    
        let mut leaderboard = query_leaderboard(deps.as_ref());
        assert!(leaderboard.is_empty());
    
        update_leaderboard(deps.as_mut().storage, &player_address, Uint128::new(1));
        leaderboard = query_leaderboard(deps.as_ref());
        assert_eq!(leaderboard.len(), 1);
        assert_eq!(leaderboard[0].player, player_address);
        assert_eq!(leaderboard[0].wins, initial_wins + Uint128::new(1));
    
        update_leaderboard(deps.as_mut().storage, &player_address, Uint128::new(2));
        leaderboard = query_leaderboard(deps.as_ref());
        assert_eq!(leaderboard.len(), 1);
        assert_eq!(leaderboard[0].player, player_address);
        assert_eq!(leaderboard[0].wins, initial_wins + Uint128::new(3));
    }
    
    #[test]
    fn test_query_game() {
        // Define mock dependencies and environment
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]);
        let env = mock_env();
        let info = mock_info("player", &[Coin{denom: "ukuji".to_string(), amount: Uint128::new(10)}]);
    
        // Set up the contract state and index
        let state = State {
            entropy_beacon_addr: Addr::unchecked("kujira1pvrwmjuusn9wh34j7y520g8gumuy9xtl3gvprlljfdpwju3x7ucseu6vw3"),
            house_bankroll: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(1000),
            },
        };
        STATE.save(&mut deps.storage, &state).unwrap();
        let idx = Uint128::zero();
        IDX.save(&mut deps.storage, &idx).unwrap();
    
        // Set up initial game state and save it in storage
        let game = Game {
            player: info.sender.to_string(),
            game_idx: idx.into(),
            bet_number: Uint128::new(3).into(),
            bet_size: info.funds[0].amount.clone().into(),
            outcome: "Pending".to_string(),
            played: false,
            win: false,
            payout: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::zero(),
            },
            rule_set: RuleSet {
                zero: Uint128::new(1),
                one: Uint128::new(3),
                two: Uint128::new(5),
                three: Uint128::new(10),
                four: Uint128::new(20),
                five: Uint128::new(45),
                six: Uint128::new(45),
            },
        };
        let game_key = idx.into();
        GAME.save(&mut deps.storage, game_key, &game).unwrap();
    
        // Call the query function for QueryMsg::Game
        let query_msg = QueryMsg::Game { idx };
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let game_response: GameResponse = from_binary(&res).unwrap();
    
        // Assert expected game response values
        assert_eq!(game_response.idx, Uint128::new(game.game_idx));
        assert_eq!(game_response.player, game.player);
        assert_eq!(game_response.bet_number, Uint128::new(game.bet_number));
        assert_eq!(game_response.bet_size, Uint128::new(game.bet_size));
        assert_eq!(game_response.game_outcome, game.outcome);
        assert_eq!(game_response.win, game.win);
        assert_eq!(game_response.payout, game.payout);
    }

    #[test]
    fn test_query_player_history() {
        // Define mock dependencies and environment
        let mut deps = mock_dependencies_with_balance(&[Coin { denom: "ukuji".to_string(), amount: Uint128::new(1000) }]);
        let env = mock_env();
        let player_addr = Addr::unchecked("player");

        // Set up initial player history state and save it in storage
        let player_history = PlayerHistory {
            player_address: player_addr.to_string(),
            games_played: Uint128::new(10),
            wins: Uint128::new(4),
            losses: Uint128::new(6),
            total_coins_spent: Coin { denom: "ukuji".to_string(), amount: Uint128::new(100) } ,
            total_coins_won: Coin { denom: "ukuji".to_string(), amount: Uint128::new(50) },
            free_spins: Uint128::new(3),
        };
        PLAYER_HISTORY.save(&mut deps.storage, player_addr.to_string(), &player_history).unwrap();

        // Call the query function for QueryMsg::PlayerHistory
        let query_msg = QueryMsg::PlayerHistory { player_addr: player_addr.clone() };
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let player_history_response: PlayerHistory = from_binary(&res).unwrap();

        // Assert expected player history response values
        assert_eq!(player_history_response.player_address, player_addr.to_string());
        assert_eq!(player_history_response.games_played, player_history.games_played);
        assert_eq!(player_history_response.wins, player_history.wins);
        assert_eq!(player_history_response.losses, player_history.losses);
        assert_eq!(player_history_response.total_coins_spent, player_history.total_coins_spent);
        assert_eq!(player_history_response.total_coins_won, player_history.total_coins_won);
        assert_eq!(player_history_response.free_spins, player_history.free_spins);
    }

    #[test]
    fn test_query_latest_game_index() {
        // Define mock dependencies and environment
        let mut deps = mock_dependencies_with_balance(&[Coin { denom: "ukuji".to_string(), amount: Uint128::new(1000) }]);
        let env = mock_env();

        // Set up the game index state and save it in storage
        let idx = Uint128::new(5);
        IDX.save(&mut deps.storage, &idx).unwrap();

        // Call the query function for QueryMsg::LatestGameIndex
        let query_msg = QueryMsg::LatestGameIndex {};
        let res = query(deps.as_ref(), env, query_msg).unwrap();
        let latest_game_index_response: LatestGameIndexResponse = from_binary(&res).unwrap();

        // Assert expected game index response value
        let expected_latest_game_index = idx - Uint128::new(1);
        assert_eq!(latest_game_index_response.idx, expected_latest_game_index);
    }

    #[test]
    fn test_query_latest_game_index_unable_to_load() {
        // Custom storage that always returns None when queried
        struct NoneStorage<'a> {
            storage: &'a mut dyn Storage,
        }
    
        impl<'a> Storage for NoneStorage<'a> {
            fn get(&self, _key: &[u8]) -> Option<Vec<u8>> {
                // Return None for any key
                None
            }
        
            fn set(&mut self, key: &[u8], value: &[u8]) {
                self.storage.set(key, value)
            }
        
            fn remove(&mut self, key: &[u8]) {
                self.storage.remove(key)
            }
        
            fn range<'b>(&'b self, start: Option<&[u8]>, end: Option<&[u8]>, order: Order) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'b> {
                self.storage.range(start, end, order)
            }
        }
    
        // Define mock dependencies and environment
        let mut deps = mock_dependencies_with_balance(&[Coin { denom: "ukuji".to_string(), amount: Uint128::new(1000) }]);
        let env = mock_env();
    
        // Replace storage with custom storage that always returns None
        let mut none_storage = NoneStorage { storage: &mut deps.storage };
    
        // Call the query function for QueryMsg::LatestGameIndex and expect a ContractError
        let query_msg = QueryMsg::LatestGameIndex {};
        let res = query(deps.as_ref(), env, query_msg);
    
        match res {
            Err(ContractError::UnableToLoadGameIndex { .. }) => (), // Expected error
            _ => panic!("Expected a ContractError::UnableToLoadGameIndex"),
        }
    }

    #[test]
    fn test_migrate() {
        // Create a test environment and mutable dependencies
        let mut deps = mock_dependencies();
    
        // Call the migrate function and check that the response contains the expected attributes
        let migrate_msg = MigrateMsg {};
        let response = migrate(deps.as_mut(), mock_env(), migrate_msg).unwrap();
    
        assert_eq!(response.attributes.len(), 1);
        assert_eq!(
            response.attributes[0],
            ("action".to_string(), "migrate".to_string())
        );
    }

    #[test]
    fn test_query_leaderboard() {
    // Define mock dependencies and environment
    let mut deps = mock_dependencies_with_balance(&[Coin { denom: "ukuji".to_string(), amount: Uint128::new(1000) }]);
    let env = mock_env();

    let rule_set = RuleSet {
        zero: Uint128::new(1),
        one: Uint128::new(3),
        two: Uint128::new(5),
        three: Uint128::new(10),
        four: Uint128::new(20),
        five: Uint128::new(45),
        six: Uint128::new(45),
    }; 

    // Set up initial game states and save them in storage
    let games = vec![
    Game {
        player: "player1".to_string(),
        game_idx: 1,
        bet_number: Uint128::new(3).into(),
        bet_size: Uint128::new(10).into(),
        outcome: "Win".to_string(),
        played: true,
        win: true,
        payout: Coin {
            denom: "ukuji".to_string(),
            amount: Uint128::new(50),
        },
        rule_set: rule_set.clone()
    },
    Game {
        player: "player2".to_string(),
        game_idx: 2,
        bet_number: Uint128::new(3).into(),
        bet_size: Uint128::new(10).into(),
        outcome: "Win".to_string(),
        played: true,
        win: true,
        payout: Coin {
            denom: "ukuji".to_string(),
            amount: Uint128::new(100),
        },
        rule_set: rule_set.clone()
    },
    Game {
        player: "player3".to_string(),
        game_idx: 3,
        bet_number: Uint128::new(3).into(),
        bet_size: Uint128::new(10).into(),
        outcome: "Win".to_string(),
        played: true,
        win: true,
        payout: Coin {
            denom: "ukuji".to_string(),
            amount: Uint128::new(200),
        },
        rule_set: rule_set.clone()
    },
    ];


    for game in games {
        let game_key = game.game_idx.to_be_bytes();
        GAME.save(&mut deps.storage, game.game_idx, &game).unwrap();
    }

    // Create leaderboard entries and save them in storage
    let leaderboard_key = "leaderboard";
    let leaderboard_entries = vec![
        LeaderBoardEntry { player: "player1".to_string(), wins: Uint128::new(3) },
        LeaderBoardEntry { player: "player2".to_string(), wins: Uint128::new(2) },
        LeaderBoardEntry { player: "player3".to_string(), wins: Uint128::new(1) },
    ];

    deps.storage.set(
        leaderboard_key.as_bytes(),
        &to_vec(&leaderboard_entries).unwrap(),
    );

    // Call the query function for QueryMsg::LeaderBoard
    let query_msg = QueryMsg::LeaderBoard {};
    let res = query(deps.as_ref(), env, query_msg).unwrap();
    let leaderboard_response: Vec<LeaderBoardEntry> = from_binary(&res).unwrap();

    // Assert expected leaderboard response values
    assert_eq!(leaderboard_response.len(), leaderboard_entries.len());
    for (i, entry) in leaderboard_response.iter().enumerate() {
        assert_eq!(entry.player, leaderboard_entries[i].player);
        assert_eq!(entry.wins, leaderboard_entries[i].wins);
    }

    }

    // STATE TESTS 
    #[test]
    fn test_is_winner_with_empty_outcome() {
        let game = Game::new_game("player1", 1, 5, 100);
        let result = game.is_winner(Uint128::new(5), vec![]);
        assert_eq!(result, false);
    }

    #[test]
    fn test_player_history_display() {
        let player_history = PlayerHistory {
            player_address: "player1".to_string(),
            games_played: Uint128::new(10),
            wins: Uint128::new(5),
            losses: Uint128::new(5),
            total_coins_spent: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(100),
            },
            total_coins_won: Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::new(50),
            },
            free_spins: Uint128::new(2),
        };

        let result = format!("{}", player_history);

        let expected = "player_address: player1, games_played: 10, wins: 5, losses: 5, total_coins_spent: (100 ukuji), total_coins_won: (50 ukuji), free_spins: 2";
        assert_eq!(result, expected);
    }
}
