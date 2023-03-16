/* mod tests {
    use crate::state::RuleSet;

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_dependencies, mock_env, mock_info, MockQuerier};
    use cosmwasm_std::{coins, QuerierWrapper, Empty, BankQuery, Coin, Uint128, BalanceResponse, QueryRequest, QueryResponse, Api, Addr};
    use hex; 


#[test]
fn addr_test() {

    let deps = mock_dependencies();
    let test1 = "kujira1ueqz5fzm27eh3wsp7n28l839g38wsw2u74lu8szehza3gp5nq8yq2vmr4s"; 
    println!("{test1}");  

    let test2 = deps.api.addr_validate("kujira1ueqz5fzm27eh3wsp7n28l839g38wsw2u74lu8szehza3gp5nq8yq2vmr4s").unwrap();
    println!("{test2}");

    // let test3 = Addr::from("kujira1ueqz5fzm27eh3wsp7n28l839g38wsw2u74lu8szehza3gp5nq8yq2vmr4s".to_string()); 
    // println!("{test3}"); 

    let x = 0;  
}

#[test]
fn test_entropy() {

    let entropy = "5526d95ed7e4116ca292a8dc7f5df6e6add7af2d285537aee0ef8bff7c2492a09cdcdc035c4ed119598bcba5bf66fdc8051db4645f97b17aa736cf0cddd5117d"; 
    
    fn is_valid_entropy(entropy: &str) -> bool {
        if entropy.len() == 128 {
            if let Ok(bytes) = hex::decode(entropy) {
                if bytes.len() == 64 {
                    return true; }}}
        false
    }

    assert!(is_valid_entropy(entropy));
}

#[test]
fn test_ruleset() {
    let rule_set = RuleSet {
        zero: Uint128::from(1u128), // 1:1
        one: Uint128::from(3u128), // 3:1
        two: Uint128::from(5u128), // 5:1
        three: Uint128::from(10u128), // 10:1
        four: Uint128::from(20u128), // 20:1
        five: Uint128::from(45u128), // 45:1
        six: Uint128::from(45u128), // 45:1 
    };

    fn validate_rules(rule_set: &RuleSet) -> bool {
        if rule_set.zero.is_zero() || rule_set.one.is_zero() || rule_set.two.is_zero()
            || rule_set.three.is_zero() || rule_set.four.is_zero() || rule_set.five.is_zero()
            || rule_set.six.is_zero() {
                return false
        }
    
        let total_ratio: u128 = rule_set.zero.u128()
            + rule_set.one.u128()
            + rule_set.two.u128()
            + rule_set.three.u128()
            + rule_set.four.u128()
            + rule_set.five.u128()
            + rule_set.six.u128();
        if total_ratio != 129 {
            return false 
        }
        
        true
    }

    assert!(validate_rules(&rule_set));
}







}



// //     #[test]
// //     fn proper_initialization() {
// //         let mut deps = mock_dependencies();

// //         let mut msg = InstantiateMsg {
// //             entropy_beacon_addr: Addr::unchecked("example_contract_address"),
// //             owner_addr: Addr::unchecked("empty_address"),
// //             win_amount: Uint128::from(0u128),
// //             token: Denom::from("USK"),
// //             play_amount: Uint128::from(1000000u128),
// //             fee_amount: Uint128::from(100000u128),
// //             rule_set: RuleSet {
// //                 zero: Uint128::from(0u128),
// //                 one: Uint128::from(1u128),
// //                 two: Uint128::from(2u128),
// //                 three: Uint128::from(3u128),
// //                 four: Uint128::from(4u128),
// //                 five: Uint128::from(5u128),
// //                 six: Uint128::from(6u128),
// //             },
// //         };

// //         // Ensure the proper contract owner address is set
// //         msg.owner_addr = Addr::unchecked("owner_address");
// //         let _verified_owner_address = deps.api.addr_validate(&msg.owner_addr.to_string()).unwrap();
// //         let info = mock_info("creator", &coins(1000, "USK"));

// //         // Ensure that the contract has been initialized successfully
// //         let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
// //         assert_eq!(0, res.messages.len());

// //         // Ensure that the state was stored properly
// //         let state = STATE.load(deps.as_ref().storage).unwrap();
// //         assert_eq!(
// //             state.entropy_beacon_addr,
// //             "kujira1xwz7fll64nnh4p9q8dyh9xfvqlwfppz4hqdn2uyq2fcmmqtnf5vsugyk7u".to_string()
// //         );

// //         // Do more assertions on the state
// //         assert_eq!(state.owner_addr, "owner".to_string());
// //         assert_eq!(state.win_amount, Uint128::from(0u128));
// //         assert_eq!(state.token, Denom::from("USK"));
// //         assert_eq!(state.play_amount, Uint128::from(1000000u128));
// //         assert_eq!(state.fee_amount, Uint128::from(100000u128));
// //         assert_eq!(
// //             state.rule_set,
// //             RuleSet {
// //                 zero: Uint128::from(0u128),
// //                 one: Uint128::from(1u128),
// //                 two: Uint128::from(2u128),
// //                 three: Uint128::from(3u128),
// //                 four: Uint128::from(4u128),
// //                 five: Uint128::from(5u128),
// //                 six: Uint128::from(6u128),
// //             }
// //         );

// //         // Ensure that the game index was initialized to zero
// //         let idx = IDX.load(deps.as_ref().storage).unwrap();
// //         assert_eq!(idx, Uint128::from(0u128));

// //     }

// //     #[test]
// //     fn test_spin_player1_win() {
// //         let mut player1_deps = mock_dependencies();
// //         let player1_env = mock_env();
// //         let player1_info1 = mock_info("player1", &coins(100, "USK"));

// //         // Create a new game, Player1 bets $USK 100 on number 0
// //         // Player should win, and recieve a payout of 1:1 (100)
// //         let player1_bet_amount = Uint128::from(100u128);
// //         let player1_bet_number = 0;

// //         // # PLAYER 1 GAME:
// //         // Id: 0, Bet 100, Sector 0, Win: 0,, Result: Win, Payout 1:1
// //         let game_id = 0u128;
// //         IDX.save(&mut player1_deps.storage, &Uint128::new(game_id))
// //             .unwrap();

// //         // Entropy beacon addr, used by all players
// //         let entropy_beacon_addr = "entropy_beacon_address".to_string();

// //         // Rule set: Used by all players
// //         let rule_set = RuleSet {
// //             // Winning odds are 1:3:5:10:20:45:45
// //             zero: Uint128::from(1u128),
// //             one: Uint128::from(3u128),
// //             two: Uint128::from(5u128),
// //             three: Uint128::from(10u128),
// //             four: Uint128::from(20u128),
// //             five: Uint128::from(45u128),
// //             six: Uint128::from(45u128),
// //         };

// //         // Player 1 state
// //         let mut player1_state = State {
// //             entropy_beacon_addr: Addr::unchecked(entropy_beacon_addr), // The entropy beacon contract address
// //             owner_addr: Addr::unchecked("owner_address"),              // The contract owner
// //             token: Denom::from("USK"), // The token used for the game
// //             house_bankroll: Coin {
// //                 denom: "USK".to_string(),
// //                 amount: Uint128::from(1000u128),
// //             }, // Initialize with 1000
// //             play_amount: player1_bet_amount.clone(), // The size of the players bet
// //             win_amount: Uint128::zero(), // The amount the player wins
// //             fee_amount: Uint128::zero(), // The amount the player pays in fees
// //             rule_set: rule_set.clone(),
// //         };

// //         //TODO: Assert that the players funds is $USK 100
// //         assert_eq!(player1_info1.funds[0].amount, Uint128::from(100u128));
// //         assert_eq!(player1_info1.funds[0].denom, "USK".to_string());

// //         // Save the player1 game in the GAME state
// //         let player1_game = Game {
// //             player: Addr::unchecked("player1"),
// //             bet: player1_bet_number,
// //             payout: Uint128::from(0u128),
// //             result: None,
// //             played: false,
// //             win: None,
// //         };
// //         GAME.save(&mut player1_deps.storage, game_id, &player1_game)
// //             .unwrap();

// //         // BET VALID?
// //         // Make sure bet is <= 10% of bank houseroll MAX, convert to float to assert correctly
// //         assert!(
// //             player1_bet_amount.to_string().parse::<f64>().unwrap()
// //                 <= player1_state
// //                     .house_bankroll
// //                     .amount
// //                     .to_string()
// //                     .parse::<f64>()
// //                     .unwrap()
// //                     * 0.1
// //         );

// //         // Send bet from player wallet to contract address
// //         let player_deposits_bet: CosmosMsg<BankMsg> = CosmosMsg::Bank(BankMsg::Send {
// //             to_address: player1_state.owner_addr.to_string(),
// //             amount: vec![Coin {
// //                 denom: "USK".to_string(),
// //                 amount: player1_bet_amount,
// //             }],
// //         });

// //         // Assert that the bet was sent to the contract address successfully
// //         assert_eq!(
// //             player_deposits_bet,
// //             CosmosMsg::Bank(BankMsg::Send {
// //                 to_address: player1_state.owner_addr.to_string(),
// //                 amount: vec![Coin {
// //                     denom: "USK".to_string(),
// //                     amount: player1_bet_amount,
// //                 }]
// //             })
// //         );

// //         // Check if owner_addr balance has increased by <player1_bet_amount>
// //         //TODO: DONT KNOW HOW TO CHECK THIS YET?
// //         // assert_eq!(player1_state.house_bankroll.amount, Uint128::new(1100u128));

// //         STATE
// //             .save(&mut player1_deps.storage, &player1_state)
// //             .unwrap();

// //         // Ensure house bankroll has been updated to $USK 1000
// //         let state = STATE.load(player1_deps.as_ref().storage).unwrap();
// //         assert_eq!(Uint128::new(1000), state.house_bankroll.amount);

// //         //TODO: Check results in this var in debug mode, works?
// //         let exe_beacon_pull = execute_entropy_beacon_pull(
// //             player1_deps.as_mut(),
// //             player1_env,
// //             player1_info1.clone(),
// //             player1_bet_amount,
// //             player1_bet_number,
// //         )
// //         .unwrap();

// //         // Faking game.result which is supposed to be generated by execute_recieve_entropy()
// //         let mut player1_game = GAME.load(player1_deps.as_ref().storage, game_id).unwrap();
// //         player1_game.result = Some(vec![0u8]);

// //         GAME.save(&mut player1_deps.storage, game_id, &player1_game)
// //             .unwrap();

// //         // Start the game
// //         let exe_spin = execute_spin(
// //             player1_deps.as_mut(),
// //             mock_env(),
// //             player1_info1.clone(),
// //             player1_bet_amount,
// //             player1_bet_number,
// //         )
// //         .unwrap();

// //         // Assert correct attributes from spin
// //         assert_eq!(
// //             exe_spin.attributes,
// //             vec![
// //                 ("game".to_string(), "0".to_string()),
// //                 (
// //                     "player".to_string(),
// //                     player1_game.player.to_string().clone()
// //                 ),
// //                 ("result".to_string(), "win".to_string()),
// //                 ("payout".to_string(), "100".to_string()),
// //             ]
// //         );

// //         // Assert that there was a message delivered with the spin, indicating the payout
// //         assert_eq!(exe_spin.messages.len(), 1);

// //         //TODO: Assert that players balance has increased by win amount (100) - Not sure if possible to check this offchain?  Maybe with a query?
// //         // assert_eq!(200, player1_info1.funds[0].amount.u128());
// //         // assert_eq!(player1_info1.funds[0].denom, "USK".to_string());

// //         // TODO: Assert that house bankroll has decreased by win amount (100)
// //         // let state = STATE.load(player1_deps.as_ref().storage).unwrap();
// //         // assert_eq!(Uint128::new(900), state.house_bankroll.amount);

// //     }

// //     #[test]
// //     fn test_spin_player1_lose() {
// //         // Entropy beacon addr, used by all players
// //         let entropy_beacon_addr = "entropy_beacon_address".to_string();

// //         // Rule set: Used by all players
// //         let rule_set = RuleSet {
// //             // Winning odds are 1:3:5:10:20:45:45
// //             zero: Uint128::from(1u128),
// //             one: Uint128::from(3u128),
// //             two: Uint128::from(5u128),
// //             three: Uint128::from(10u128),
// //             four: Uint128::from(20u128),
// //             five: Uint128::from(45u128),
// //             six: Uint128::from(45u128),
// //         };

// //         // # PLAYER 1 GAME:
// //         // Id: 0, Bet 100, Sector 0, Win: 1, Result: Lose Payout 1:1
// //         let mut gameId = 0u128;

// //         let mut player1_deps = mock_dependencies();
// //         IDX.save(&mut player1_deps.storage, &Uint128::new(gameId))
// //             .unwrap();

// //         // Player 1 state
// //         let mut player1_state = State {
// //             entropy_beacon_addr: Addr::unchecked(entropy_beacon_addr), // The entropy beacon contract address
// //             owner_addr: Addr::unchecked("owner_address"),              // The contract owner
// //             token: Denom::from("USK"), // The token used for the game
// //             house_bankroll: Coin {
// //                 denom: "USK".to_string(),
// //                 amount: Uint128::from(1000u128),
// //             }, // Initialize with 1000
// //             play_amount: Uint128::from(0u128), // The size of the players bet
// //             win_amount: Uint128::zero(), // The amount the player wins
// //             fee_amount: Uint128::zero(), // The amount the player pays in fees
// //             rule_set: rule_set.clone(),
// //         };

// //         // Create a new game, Player1 bets $USK 100 on number 0
// //         // Player should win, and recieve a payout of 1:1 (100)
// //         let player1 = "player1".to_string();
// //         let player1_bet_amount = Uint128::from(100u128);
// //         let player1_bet_number = 0;
// //         let player1_info1 = mock_info(&player1, &coins(100, "USK"));

// //         // Save the player1 game in the GAME state
// //         let player1_game = Game {
// //             player: Addr::unchecked("player1"),
// //             bet: player1_bet_number,
// //             payout: Uint128::from(0u128),
// //             result: None,
// //             played: false,
// //             win: None,
// //         };
// //         GAME.save(&mut player1_deps.storage, gameId, &player1_game)
// //             .unwrap();

// //         player1_state.play_amount = player1_bet_amount.clone();

// //         // Make sure bet is <= 10% of bank houseroll MAX, convert to float to assert correctly
// //         assert!(
// //             player1_bet_amount.to_string().parse::<f64>().unwrap()
// //                 <= player1_state
// //                     .house_bankroll
// //                     .amount
// //                     .to_string()
// //                     .parse::<f64>()
// //                     .unwrap()
// //                     * 0.1
// //         );

// //         STATE
// //             .save(&mut player1_deps.storage, &player1_state)
// //             .unwrap();

// //         // Ensure house bankroll has been updated to $USK 1000
// //         let state = STATE.load(player1_deps.as_ref().storage).unwrap();
// //         assert_eq!(Uint128::new(1000), state.house_bankroll.amount);

// //         //TODO: Check results in this var in debug mode, works?
// //         let exe_beacon_pull = execute_entropy_beacon_pull(
// //             player1_deps.as_mut(),
// //             mock_env(),
// //             player1_info1.clone(),
// //             player1_bet_amount,
// //             player1_bet_number,
// //         )
// //         .unwrap();

// //         // Faking game.result created by entropy callback
// //         let simulated_entropy_result = vec![1u8];

// //         //TODO: Find some other way to test this? Cannot get EntropyCallbackMsg in debug mode?
// //         /*   let exe_recieve_entropy = execute_recieve_entropy(
// //             deps.as_mut(),
// //             mock_env(),
// //             player1_info1.clone(),
// //             data, // NEED ENTROPY CB DATA
// //         ).unwrap(
// //         );  */
// //         // Faking game.result which is supposed to be generated by execute_recieve_entropy()
// //         let mut player1_game = GAME.load(player1_deps.as_ref().storage, gameId).unwrap();

// //         // player1_game.result = Some(get_outcome_from_entropy(&entropy_gen_test));
// //         player1_game.result = Some(simulated_entropy_result.clone());
// //         GAME.save(&mut player1_deps.storage, gameId, &player1_game)
// //             .unwrap();

// //         // Start the game
// //         let exe_spin = execute_spin(
// //             player1_deps.as_mut(),
// //             mock_env(),
// //             player1_info1.clone(),
// //             player1_bet_amount,
// //             player1_bet_number,
// //         )
// //         .unwrap();

// //         // Assert correct attributes from spin
// //         assert_eq!(
// //             exe_spin.attributes,
// //             vec![
// //                 ("game".to_string(), "0".to_string()),
// //                 ("player".to_string(), player1.clone()),
// //                 ("result".to_string(), "lose".to_string()),
// //             ]
// //         );

// //         //TODO: Assert player balance is updated correctly
// //         //TODO: Assert bankroll balance is updated correctly
// //         //TODO: HOW DO WE DO THIS? cw_multi_test?
// //         //TODO: Check for submessage in result of the execute call?
// //     }

// //     #[test]
// //     fn test_execute_validate_bet() {
// //         let mut deps = mock_dependencies();

// //         // Player 1 bets 100 USK on number 0
// //         let info = mock_info("Player1", &coins(100, "USK"));
// //         let bet_number = 0;

// //         let state = State {
// //             entropy_beacon_addr: Addr::unchecked("entropy_beacon_addr".to_string()),
// //             owner_addr: Addr::unchecked("owner"),
// //             token: Denom::from("USK"), // The token used for the game
// //             house_bankroll: Coin {
// //                 denom: "USK".to_string(),
// //                 amount: Uint128::from(1234u128),
// //             }, // Initialize with 1000
// //             play_amount: info.funds[0].amount, // The size of the players bet
// //             win_amount: Uint128::zero(), // The amount the player wins
// //             fee_amount: Uint128::zero(), // The amount the player pays in fees
// //             rule_set: RuleSet {
// //                 // Winning odds are 1:3:5:10:20:45:45
// //                 zero: Uint128::from(1u128),
// //                 one: Uint128::from(3u128),
// //                 two: Uint128::from(5u128),
// //                 three: Uint128::from(10u128),
// //                 four: Uint128::from(20u128),
// //                 five: Uint128::from(45u128),
// //                 six: Uint128::from(45u128),
// //             },
// //         };

// //         // Check that only one denom was sent
// //         let coin = one_coin(&info).unwrap();

// //         // Check that the denom is the same as the token in the state
// //         assert!(state.token == Denom::from(coin.denom));

// //         // Check that the amount is the same as the play_amount in the state
// //         assert!(state.play_amount == coin.amount);

// //         // Check that the amount the player is betting is >= 1
// //         assert!(state.play_amount >= Uint128::from(1u128));

// //         // Check that the bet amount is <= 10% of bank houseroll MAX, round house bankroll to nearest integer
// //         assert!(
// //             state.play_amount
// //                 <= state.house_bankroll.amount.checked_div(Uint128::new(10)).unwrap()
// //         );

// //         // Check that the player_bet_number is between 0 and 6
// //         assert!(bet_number <= 6u8);

// //         let exe_validate_bet = execute_validate_bet(
// //             &deps.as_mut(),
// //             info,
// //             state.play_amount,
// //             bet_number);

// //         // Check that the response is OK from the execute_validate_bet function
// //         assert_eq!(exe_validate_bet, true);
// //     }

// // }
  */