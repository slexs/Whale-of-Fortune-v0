use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use entropy_beacon_cosmos::EntropyCallbackMsg;
use schemars::JsonSchema;

#[cw_serde]
pub struct InstantiateMsg {
    pub entropy_beacon_addr: Addr,
}

#[cw_serde]
pub struct EntropyCallbackData {
    pub original_sender: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    Spin { bet_number: Uint128 },
    ReceiveEntropy(EntropyCallbackMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GameResponse)]
    Game { idx: Uint128 },
}

#[cw_serde]
pub struct GameResponse {
    pub idx: Uint128,
    pub outcome: String,
}

#[cw_serde]
pub struct MigrateMsg {}
