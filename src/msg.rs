use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Coin, Addr};
use crate::state::State;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub strike_price: Coin,
    pub expiration_date: u64,
    pub buysell: bool, // buy is 0 sell is 1
    pub putcall: bool, // call is 0 put is 1
    pub quantity: u64,
    pub liq_pool: Addr, 
    pub conv_coin: String,
    pub opt_coin: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    InitOption { recipient: Addr },
    Expires {},
    SellOption { owner: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    Config {},
}

// We define a custom struct for each query response
//#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub type ConfigResponse = State;
