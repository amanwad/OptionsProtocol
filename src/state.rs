use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Item;



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub expiration_date: u64,
    pub putcall: bool,
    pub buysell: bool,
    pub strike_price: Coin,
    pub quantity: u64,
    pub liquidity: Addr,
    pub sent: Vec<Coin>,
    pub conv_coin: String,
    pub opt_coin: String
}


pub const STATE: Item<State> = Item::new("state");
