use crate::state::{LatestGameIndexResponse, LeaderBoardEntry, PlayerHistory};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Uint128};
use entropy_beacon_cosmos::EntropyCallbackMsg;

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
    FreeSpin { bet_number: Uint128 },
    ReceiveEntropy(EntropyCallbackMsg),
    AdminExecuteChangeEntropyAddr { new_addr: Addr },
    AdminExecuteChangeFreeSpinThreshold { new_threshold: Uint128 },
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

    #[returns(Vec<LeaderBoardEntry>)] // Modify return type according to the new struct
    LeaderBoard {},
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
