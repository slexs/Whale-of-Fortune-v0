#[allow(unused_imports)]
pub mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, mock_dependencies_with_balance};
    use cosmwasm_std::{Uint128, Coin, Addr};
    use crate::contract::execute;
    use crate::helpers::{calculate_payout, get_outcome_from_entropy, execute_validate_bet};
    use crate::state::{RuleSet, State, Game, PLAYER_HISTORY, PlayerHistory};
    use crate::msg::{ExecuteMsg, InstantiateMsg};


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