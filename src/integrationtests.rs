#[cfg(test)]
mod tests {
    use crate::contract::{instantiate, execute};

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Empty, Addr, DepsMut, Uint128};
    use crate::msg::{InstantiateMsg, ExecuteMsg};


    #[test]
    fn test_instantiate_and_execute() {
        let mut deps = mock_dependencies();
        let creator = "player";
        let instantiate_msg = InstantiateMsg {
            entropy_beacon_addr: Addr::unchecked("entropyaddr".to_string()),
        };
        let init_info = mock_info(creator, &coins(10000, "ukuji"));

        // Instantiate the contract
        let res = instantiate(deps.as_mut(), mock_env(), init_info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Execute the contract
        let execute_info = mock_info(creator, &coins(2000, "ukuji"));
        let execute_msg = ExecuteMsg::Spin { bet_number: Uint128::new(1), };
        let execute_res = execute(deps.as_mut(), mock_env(), execute_info, execute_msg).unwrap();
        
        // Check the expected results
        // ...

        let x = 0; 
    }
}
