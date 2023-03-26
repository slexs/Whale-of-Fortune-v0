use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Coin};
use entropy_beacon_cosmos::EntropyCallbackMsg;
use crate::state::{PlayerHistory, LatestGameIndexResponse};

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
    FreeSpin { bet_number: Uint128 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GameResponse)]
    Game { idx: Uint128 },

    #[returns(PlayerHistory)]
    PlayerHistory { player_addr: Addr },

    #[returns(LatestGameIndexResponse)]
    LatestGameIndex {}, 
}

#[cw_serde]
pub struct GameResponse {
    pub idx: Uint128,
    pub player: String,
    pub bet_number: Uint128,
    pub bet_size: Uint128,
    pub played: bool, 
    pub payout: Coin,
    pub game_outcome: String,
    pub win: bool, 
}



#[cw_serde]
pub struct MigrateMsg {}
