#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
// use cosmwasm_std::CosmosMsg::{Bank};
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, Uint128};

use crate::error::ContractError;
use crate::msg::{
    EntropyCallbackData, ExecuteMsg, GameResponse, InstantiateMsg, MigrateMsg, QueryMsg,
};
use crate::state::{Config, Game, RuleSet, CONFIG, GAME, IDX};

use sha2::{Digest, Sha256};
use cw_utils::one_coin;
use entropy_beacon_cosmos::{CalculateFeeQuery, EntropyCallbackMsg, EntropyRequest};
use kujira::denom::Denom;
use cw2::set_contract_version;

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:Spin-the-whale";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Our [`InstantiateMsg`] contains the address of the entropy beacon contract.
/// We save this address in the contract state.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // entropy beacon addr TESTNET harpoon-4
    let entropy_beacon_addr = "kujira1xwz7fll64nnh4p9q8dyh9xfvqlwfppz4hqdn2uyq2fcmmqtnf5vsugyk7u"; 

    // entropy beacon addr MAINNET kaiyo-1
    // let entropy_beacon_addr = "kujira1x623ehq3gqx9m9t8asyd9cgehf32gy94mhsw8l99cj3l2nvda2fqrjwqy5"; 

    // Validate the entropy beacon addr 
    let validated_entropy_beacon_addr = deps.api.addr_validate(entropy_beacon_addr)?;

    // validate the owner's address
    let validated_owner_address: Addr = deps.api.addr_validate(info.sender.as_ref())?;

    // Initialize Config
    let config = Config {
        entropy_beacon_addr: validated_entropy_beacon_addr,
        owner_addr: validated_owner_address,
        house_bankroll: Coin { // Init house bankroll to zero ukuji 
            denom: "ukuji".to_string(),
            amount: Uint128::zero(),
        },
        token: Denom::from("ukuji"), // Init token to ukuji
        fee_amount: Uint128::zero(),
        rule_set: RuleSet { // Payout ratios
            zero: Uint128::from(1u128), // 1:1
            one: Uint128::from(3u128), // 3:1
            two: Uint128::from(5u128), // 5:1
            three: Uint128::from(10u128), // 10:1
            four: Uint128::from(20u128), // 20:1
            five: Uint128::from(45u128), // 45:1
            six: Uint128::from(45u128), // 45:1 
        },
    };

    // Save the initialized config to storage 
    CONFIG.save(deps.storage, &config)?;
    
    // Save the initialized game index 0 to storage
    IDX.save(deps.storage, &Uint128::zero())?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", config.owner_addr.to_string())
        .add_attribute("entropy_beacon_addr", config.entropy_beacon_addr.to_string())
        .add_attribute("house_bankroll", config.house_bankroll.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // #STEP 1:
        // Validate player's bet amount and number
        // and handle requesting entropy from the beacon.
        ExecuteMsg::Pull { bet_number } => {
            // Load the game config 
            let config = CONFIG.load(deps.storage)?;
        
            // Check that players bet amount is <= 10% of house bankroll, bet num [0, 6], denom etc
            if !execute_validate_bet(
                &deps, 
                &env, 
                info.clone(), 
                info.funds[0].amount, 
                bet_number) {
                    return Err(ContractError::InvalidBet {});
                }
        
            // Get the current gameID
            let idx = IDX.load(deps.storage)?;
        
            // Create a new game state for this game 
            let game = Game {
                player: info.sender.clone(),
                bet_number: bet_number,
                bet_size: info.funds[0].amount,
                payout: Uint128::zero(), // Payout not yet decided in this step
                result: None,
                played: false,
                win: None,
                game_id: idx,
                entropy_requested: false, 
            };
        
            // Save the game state to the contract
            GAME.save(deps.storage, idx.u128(), &game)?;
        
            let callback_gas_limit = 100_000u64;
        
            let beacon_fee = CalculateFeeQuery::query(deps.as_ref(), callback_gas_limit, config.entropy_beacon_addr.clone())?;
        
            // // Create a request for entropy from the Beacon contract
            // let mut msgs = vec![EntropyRequest {
            //     callback_gas_limit, 
            //     callback_address: env.contract.address.clone(),
            //     funds: vec![Coin {
            //         denom: config.token.to_string(),
            //         amount: Uint128::from(beacon_fee),
            //     }],
            //     callback_msg: EntropyCallbackData {
            //         original_sender: info.sender,
            //         game: idx,
            //     },
            // }.into_cosmos(config.entropy_beacon_addr)?];
        
            // If there is a fee, send it to the fee address
            // if !config.fee_amount.is_zero() {
            //     msgs.push(CosmosMsg::Bank(BankMsg::Send {
            //         to_address: kujira::utils::fee_address().to_string(),
            //         amount: config.token.coins(&config.fee_amount),
            //     }))
            // };
            // if !config.fee_amount.is_zero() {
            //     CosmosMsg::Bank(BankMsg::Send {
            //         to_address: kujira::utils::fee_address().to_string(),
            //         amount: config.token.coins(&config.fee_amount),
            //     }) 
            // };

            // Transfer player bet amount to the contract address
            let _player_deposit_msg: BankMsg = BankMsg::Send {
                to_address: env.contract.address.to_string(),
                amount: config.token.coins(&game.bet_size),
            };
        
            Ok(Response::new().add_message(
                EntropyRequest {
                    callback_gas_limit,
                    callback_address: env.contract.address,
                    funds: vec![Coin {
                        denom: "ukuji".to_string(),
                        amount: Uint128::from(1000u128),
                    }],
                    // A custom struct and data we define for callback info.
                    // You should change this callback message struct to match the information your contract needs.
                    callback_msg: EntropyCallbackData {
                        original_sender: Addr::unchecked(info.sender),
                        game: idx, 
                    },
                }
                .into_cosmos(config.entropy_beacon_addr)?,
            ))

            // // Response to the contract caller
            // Ok(Response::new()
            // // .add_attribute("game", idx)
            // // .add_attribute("player", game.player)
            // .add_messages(msgs))
            // // .add_message(_player_deposit_msg))
        
        },

        // #STEP 2:
        // Handle receiving entropy from the beacon.
        ExecuteMsg::ReceiveEntropy(data) => {
            
            // Load the game state from the contract
            let config = CONFIG.load(deps.storage)?;
            let idx = IDX.load(deps.storage)?;
            
            // Load game state with current game index
            let mut game = GAME.load(deps.storage, idx.u128())?;
            
            // Set entropy_requested flag to true (DEBUG)
            game.entropy_requested = true;
            
            // save game state with entropy requested flag set to true 
            GAME.save(deps.storage, idx.u128(), &game)?; 

            let mut game = GAME.load(deps.storage, idx.u128())?;

            // Get the address of the entropy beacon
            let beacon_addr = config.entropy_beacon_addr;

            // IMPORTANT: Verify that the callback was called by the beacon, and not by someone else.
            if info.sender != beacon_addr {
                return Err(ContractError::InvalidEntropyCallback {});
            }

            //* IMPORTANT: Verify that the original requester for entropy is trusted (e.g.: this contract)
            if data.requester != env.contract.address {
                return Err(ContractError::InvalidEntropyRequester {});
            }

            // The callback data has 64 bytes of entropy, in a Vec<u8>.
            let entropy = data.entropy;

            // We can parse out our custom callback data from the message.
            let callback_data = data.msg;
            let callback_data: EntropyCallbackData = from_binary(&callback_data)?;

            // gets a result (0-6) from the entropy, and sets game state to played
            game.result = Some(get_outcome_from_entropy(&entropy));
            let result = game.result.clone().unwrap();

            // Ensure that the result is valid (less than 6)
            if result[0] > 6u8 {
                return Err(ContractError::InvalidGameResult {msg: format!("result:{}", result[0])});
            }

            game.played = true;

            // GAME.save(deps.storage, idx.u128(), &game)?;

            //TODO: Do the Spin() logic inside this function 
            // Calculate the possible payout for the player
            let calculated_payout = calculate_payout(game.bet_size.clone(), result[0], config.rule_set);

            // If the player won, set the win flag to true and send the payout to the player
            if game.win(game.bet_number) {
                let payout_coin = Coin {
                    denom: config.token.to_string(),
                    amount: calculated_payout,
                };

                // Set game win flag to true
                game.win = Some(true); 

                // Send the payout to the players address
                let payout_msg = BankMsg::Send {
                    to_address: game.player.to_string(), 
                    amount: vec![payout_coin],
                };

                game.played = true;
                game.payout = calculated_payout;
                game.result = Some(result.clone());
                GAME.save(deps.storage, idx.u128(), &game)?;

                // Increment GameID for the next game 
                IDX.save(deps.storage, &(idx + Uint128::from(1u128)))?;

                return Ok(Response::new()
                .add_message(payout_msg) 
                .add_attribute("game", callback_data.game) 
                .add_attribute("player", game.player)
                .add_attribute("payout", calculated_payout.to_string())
                .add_attribute("result", result[0].to_string())
                .add_attribute("callbackdata.game", callback_data.game.to_string()))
            } 
            // Player did NOT win the game (loss)
            else {
                game.played = true; 
                game.played = true;
                game.win = Some(false);
                game.payout = Uint128::zero();
                game.result = Some(result);
                GAME.save(deps.storage, idx.u128(), &game)?;

                // Increment gameID for the next game
                IDX.save(deps.storage, &(idx + Uint128::from(1u128)))?;

                return Ok(Response::new()
                    .add_attribute("game", idx.u128().to_string())
                    .add_attribute("player", game.player.to_string())
                    .add_attribute("result", "lose"));

                }
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
                result: game.result.as_ref().map(|x| x.clone()),
                win: game.win(game.bet_size),
                entropy_requested: game.entropy_requested,
            })
        }
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;
    config.fee_amount = msg.fee_amount;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

// Validate the players bet amount and number
pub fn execute_validate_bet(
    deps: &DepsMut,
    env: &Env,
    info: MessageInfo,
    player_bet_amount: Uint128,
    player_bet_number: Uint128,
) -> bool {

    let mut config = CONFIG.load(deps.storage).unwrap();

    // Get the balance of the house bankroll (contract address balance)
    let bankroll_balance = match 
    deps.querier
    .query_balance(
        env.contract.address.to_string()
        , "ukuji".to_string()) {
            Ok(balance) => balance,
            Err(_) => return false,
        };

    config.house_bankroll = bankroll_balance.clone(); 

    // Check that the players bet number is between 0 and 6
    if player_bet_number > Uint128::new(6) {
        return false;
    }

    // Check that only one denom was sent
    let coin = match one_coin(&info) {
        Ok(coin) => coin,
        Err(_) => return false,
    };

    // Check that the denom is the same as the token in the bankroll ("ukuji")
    if coin.denom != bankroll_balance.denom {
        return false;
    }

    // Get the balance of the house bankroll (contract address balance)
    // let bankroll_balance = deps.querier
    // .query_balance(
    //     env.contract.address.to_string()
    //     , "ukuji".to_string()
    //     )?;


    // Make sure the player's bet_amount does not exceed 10% of house bankroll
    if player_bet_amount
        > bankroll_balance.amount
        .checked_div(Uint128::new(10))
        .unwrap() {
        return false;
    }

    true
}

// Calculate the payout amount for a given bet
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

// Take the entropy and return a random number between 0 and 6
pub fn get_outcome_from_entropy(entropy: &[u8]) -> Vec<u8> {
    // Hash the input entropy using SHA256
    let mut hasher = Sha256::new();
    hasher.update(entropy);
    let hash_result = hasher.finalize();

    // Use the last byte of the hash as the random number
    let random_byte = hash_result[hash_result.len() - 1];

    // Map the random byte to a number between 0 and 6
    let outcome = random_byte % 7;
    vec![outcome]
}
