#[allow(unused_imports)]
pub mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, mock_dependencies_with_balance};
    use cosmwasm_std::{Uint128, Coin, Addr, Response, coins, from_binary, to_binary, WasmMsg};
    use cw2::{set_contract_version, CONTRACT, get_contract_version};
    use entropy_beacon_cosmos::beacon::RequestEntropyMsg;
    use entropy_beacon_cosmos::provide::ActiveRequestsQuery;
    use entropy_beacon_cosmos::{EntropyRequest, EntropyCallbackMsg, CalculateFeeQuery};
    use entropy_beacon_cosmos::msg::QueryMsg as BeaconQueryMsg;
    use entropy_beacon_cosmos::msg::ExecuteMsg as BeaconExecuteMsg;
    use crate::contract::{execute, instantiate};
    use crate::helpers::{calculate_payout, get_outcome_from_entropy, execute_validate_bet};
    use crate::state::{RuleSet, State, Game, PLAYER_HISTORY, PlayerHistory, STATE, IDX, GAME};
    use crate::msg::{ExecuteMsg, InstantiateMsg, EntropyCallbackData};


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

        let callback_gas_limit = 100_000u64;
        let beacon_fee = 0u128; 

        // Create an EntropyRequest submessage
        let entropy_request =  EntropyRequest {
            callback_gas_limit,
            callback_address: env.contract.address,
            funds: vec![Coin {
                denom: "ukuji".to_string(),
                amount: Uint128::from(beacon_fee),
            }],
                callback_msg: EntropyCallbackData {
                    original_sender: info.sender,
            },
        }; 

        // Check that the EntropyRequest callback address is the same as the contract address 
        assert_eq!(
            entropy_request.callback_address,
            env.contract.address.clone()
        );
        // Check that the EntropyRequest funds are correct
        assert_eq!(entropy_request.funds, vec![Coin::new(Uint128::zero().into(), "ukuji".to_string())]);

        // Check that the EntropyRequest original sender is correct 
        assert_eq!(
            entropy_request.callback_msg,
            EntropyCallbackData {
                original_sender: info.sender.clone(),
            }
        );

        // Check that the EntropyRequest submessage calls the expected function
        let expected_entropy_callback = WasmMsg::Execute {
            contract_addr: state.entropy_beacon_addr.clone().into_string(),
            msg: to_binary(&BeaconExecuteMsg::RequestEntropy(EntropyRequest {
                callback_gas_limit,
                callback_address: env.contract.address.clone(),
                funds: vec![Coin {
                    denom: "ukuji".to_string(),
                    amount: Uint128::from(beacon_fee),
                }],
                callback_msg: EntropyCallbackData {
                    original_sender: info.sender.clone(),
                },
            }))
            .unwrap(),
            funds: vec![Coin { amount: Uint128::new(10), denom: "ukuji".to_string() }],
        };
        
        assert_eq!(submsg.msg.as_ref().unwrap().msg, expected_entropy_callback.into_binary().unwrap());
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
                        win_loss_ratio: Uint128::zero(),
                        total_coins_spent: Coin {
                            denom: "ukuji".to_string(),
                            amount: Uint128::zero(),
                        },
                        total_coins_won: Coin {
                            denom: "ukuji".to_string(),
                            amount: Uint128::zero(),
                        },
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
    }

    #[test]
    fn test_get_outcome_from_entropy() {

        let rule_set =  RuleSet {
            zero: Uint128::new(24),
            one: Uint128::new(12),
            two: Uint128::new(8),
            three: Uint128::new(4),
            four: Uint128::new(2),
            five: Uint128::new(1),
            six: Uint128::new(1),
        }; 

        // Valid, entropy will result in result = 0
        let entropy = hex::decode("68b7cfd0fcfd3564359318426bea7f203ebc8687bda140645d60caaf79b6b18b9e8d9c93e62f2b2e138c520253b96c23800b2f82274586a4b5f246a3479a5715").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 1);
        assert!(outcome[0] <= 6);
        assert!(outcome[0] == 0);

        // Valid, entropy will result in result = 4
        let entropy = hex::decode("54c86044dfdd18902279243ce80741ab186cba4027c137fab649b861fb328da77b3bebe62783c76b96fc34381a855f9383d9d20ff83fbc3ecbab7c90d1b597ba").unwrap();
        let outcome = get_outcome_from_entropy(&entropy, &rule_set);
        assert!(outcome.len() == 1);
        assert!(outcome[0] <= 6);
        assert!(outcome[0] == 4);

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
    fn test_validate_bet() {
        
        // Valid bet 
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]); 
        let env = mock_env();
        let info = mock_info("addr0000", &[Coin{denom: "ukuji".to_string(), amount: Uint128::new(10)}]);
        let bet_amount = Uint128::new(10); 
        let bet_number = Uint128::new(5); 
        assert_eq!(execute_validate_bet(&deps.as_mut(), &env, info, bet_amount, bet_number), true);

        // Invalid bet, wrong bet number!
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]); 
        let env = mock_env();
        let info = mock_info("addr0000", &[Coin{denom: "ukuji".to_string(), amount: Uint128::new(10)}]);
        let bet_amount = Uint128::new(10); 
        let bet_number = Uint128::new(7); // WRONG BET NUMBER
        assert_eq!(execute_validate_bet(&deps.as_mut(), &env, info, bet_amount, bet_number), false);

        // Invalid bet, not enough balance!
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]); 
        let env = mock_env();
        let info = mock_info("addr0000", &[Coin{denom: "ukuji".to_string(), amount: Uint128::new(10)}]);
        let bet_amount = Uint128::new(100); // Bet size larger than balance 
        let bet_number = Uint128::new(2); 
        assert_eq!(execute_validate_bet(&deps.as_mut(), &env, info, bet_amount, bet_number), false);

        // Invalid bet, wrong denom!
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]);
        let env = mock_env();
        let info = mock_info("addr0000", &[Coin{denom: "wrong_denom".to_string(), amount: Uint128::new(10)}]);
        let bet_amount = Uint128::new(10);
        let bet_number = Uint128::new(2);
        assert_eq!(execute_validate_bet(&deps.as_mut(), &env, info, bet_amount, bet_number), false);

        // Invalid bet,  bet size is > 1% of house bankroll 
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]);
        let env = mock_env();
        let info = mock_info("addr0000", &[Coin{denom: "ukuji".to_string(), amount: Uint128::new(100)}]);
        let bet_amount = Uint128::new(100); // Bet size larger than 1% of house bankroll
        let bet_number = Uint128::new(2);
        assert_eq!(execute_validate_bet(&deps.as_mut(), &env, info, bet_amount, bet_number), false);

        // Invalid bet, bankroll size is 0
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(0)}]);
        let env = mock_env();
        let info = mock_info("addr0000", &[Coin{denom: "ukuji".to_string(), amount: Uint128::new(10)}]);
        let bet_amount = Uint128::new(10);
        let bet_number = Uint128::new(2);
        assert_eq!(execute_validate_bet(&deps.as_mut(), &env, info, bet_amount, bet_number), false);

    }

    
}