#[allow(unused_imports)]
pub mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, mock_dependencies_with_balance};
    use cosmwasm_std::{Uint128, Coin, DepsMut, Env, MessageInfo};
    use crate::helpers::{calculate_payout, get_outcome_from_entropy, execute_validate_bet};
    use crate::msg::ExecuteMsg;
    use crate::state::{RuleSet, PLAYER_HISTORY, PlayerHistory};

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

    #[test]
    fn test_create_and_update_player_history() {
        let mut deps = mock_dependencies_with_balance(&[Coin{denom: "ukuji".to_string(), amount: Uint128::new(1000)}]); 
        let env = mock_env();
        let info = mock_info("addr0000", &[Coin { denom: "ukujis".to_string(), amount: Uint128::new(10) }]);

        let player_history = PLAYER_HISTORY.may_load(&deps.storage, info.sender.to_string()).unwrap();
            if player_history.is_none() {
                let new_player_history = PlayerHistory {
                    player: info.sender.to_string(), 
                    games_played: Uint128::new(0),
                    games_won: Uint128::new(0),
                    games_lost: Uint128::new(0),
                    total_winnings: Uint128::new(0),
                    total_losses: Uint128::new(0),
                    loyalty_points: Uint128::new(0),
                };

                PLAYER_HISTORY.save(&mut deps.storage, info.sender.to_string(), &new_player_history).unwrap();
            }

        // load player history to check the contents 
        let player_history = PLAYER_HISTORY.load(&deps.storage, info.sender.to_string()).unwrap();

        assert_eq!(player_history.player, info.sender.to_string());
        assert_eq!(player_history.games_played, Uint128::new(0));
        assert_eq!(player_history.games_won, Uint128::new(0));
        assert_eq!(player_history.games_lost, Uint128::new(0));
        assert_eq!(player_history.total_winnings, Uint128::new(0));
        assert_eq!(player_history.total_losses, Uint128::new(0));
        assert_eq!(player_history.loyalty_points, Uint128::new(0));
        

    }

    
}