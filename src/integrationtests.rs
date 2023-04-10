/* #[cfg(test)]
mod tests {
    use crate::contract::{instantiate, execute};
    use crate::state::{IDX, GAME};

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, mock_dependencies_with_balance, mock_dependencies_with_balances};
    use cosmwasm_std::{coins, Empty, Addr, DepsMut, Uint128, Coin};
    use crate::msg::{InstantiateMsg, ExecuteMsg};


    #[test]
    fn test_instantiate_and_execute() {
        let mut deps = mock_dependencies_with_balances(&[
            ("player", &coins(2000, "ukuji")),
            ("cosmos2contract", &coins(100000000, "ukuji")),
        ]);

        let env = mock_env(); 
        
        let instantiate_msg = InstantiateMsg {
            entropy_beacon_addr: Addr::unchecked("entropyaddr".to_string()),
        };

        let creator = "player";
        let init_info = mock_info(creator, &coins(10000, "ukuji"));

        // Instantiate the contract
        let res = instantiate(deps.as_mut(), mock_env(), init_info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        
        

        // Execute the contract
        let execute_info = mock_info(creator, &coins(2000, "ukuji"));
        let execute_msg = ExecuteMsg::Spin { bet_number: Uint128::new(1), };
        let execute_res = execute(deps.as_mut(), mock_env(), execute_info, execute_msg).unwrap();

        let idx = IDX.load(deps.as_ref().storage).unwrap();
        let game = GAME.load(deps.as_ref().storage, idx.into()).unwrap();
        
        // Check the expected results
        // 1. Query the contract state to get the total bet amount
        
        let total_bet_amount: Uint128 = game.bet_size.into(); 
        assert_eq!(total_bet_amount, Uint128::new(2000));

        // 2. Check if the event was emitted with the expected values
    
        let bet_amount_event = execute_res
            .attributes
            .iter()
            .find(|attr| attr.key == "bet_amount" && attr.value == "2000")
            .expect("bet_amount event not found");

        

        }
}
 */