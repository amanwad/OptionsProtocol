#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, coins, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, BankMsg};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};


const CONTRACT_NAME: &str = "crates.io:option-protocol";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if msg.expiration_date <= _env.block.height {
        return Err(ContractError :: Unauthorized {});
    }
    let state = State {
        owner: info.sender.clone(),
        expiration_date: msg.expiration_date,
        putcall: msg.putcall,
        buysell: msg.buysell,
        strike_price: msg.strike_price,
        quantity: msg.quantity,
        liquidity: msg.liq_pool,
        sent: info.funds,
        conv_coin: msg.conv_coin,
        opt_coin: msg.opt_coin
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::InitOption { recipient } => try_initoption(deps, info, env, recipient),
        ExecuteMsg::Expires {} => try_expires(deps, info, env),
        ExecuteMsg::SellOption { owner } => try_selloption(deps, info, env, owner),
    }
}


pub fn try_initoption(deps: DepsMut, info: MessageInfo, env: Env, recipient: Addr) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    
    let cur_price = 8.01_f64; // from price oracle
    let vol = 1.00; // volatility from price oracle
    const RATE:f64 = 0.0021; // constant risk free rate of return
    
    let expire = state.expiration_date as f64;
    let amount = (cur_price / state.strike_price.amount) as f64;
    
    let d1 = (((cur_price / amount).ln()) + expire * (RATE + ((vol * vol) / 2.0))) / (vol * expire.sqrt());
    let d2 = (((cur_price / amount).ln()) + expire * (RATE - ((vol * vol) / 2.0))) / (vol * expire.sqrt());

    let n = Uniform::new(0.0, 1.0).unwrap();

    if state.putcall {
        let call_price = (cur_price * n.cdf(d1)) - (amount * (-RATE * expire).exp() * n.cdf(d2));
        let totalpremium = call_price * (state.quantity as f64);
        let sendpremium = coins(totalpremium, state.conv_coin);
        if state.buysell {
            let mut res = Response::new();
            res.add_message(BankMsg::Send {
                from_address: state.owner.to_string(),
                to_address: state.liquidity.to_string(),
                amount: sendpremium,
                });
            //transfer premium in from recipient to liq pool
        }
        else {
            let mut res = Response::new();
            res.add_message(BankMsg::Send {
                from_address: state.liquidity.to_string(),
                to_address: state.owner.to_string(),
                amount: sendpremium,
                });
            let collateral = coins(state.quantity.into(), state.opt_coin);
            res.add_message(BankMsg::Send {
                from_address: state.owner.to_string(),
                to_address: state.liquidity.to_string(),
                amount: collateral,
                });
            // transfer premium in stblecoins from liq pool to recipient && transfer collateral from recipient to liq pool (quantity coins)
        }
    }
    else {
        let put_price = (amount * (-RATE * expire).exp() * n.cdf(-d2)) - (cur_price * n.cdf(-d1));
        let totalpremium = put_price * (state.quantity as f64);
        let sendpremium = coins(totalpremium, state.conv_coin);
        if state.buysell {
            let mut res = Response::new();
            res.add_message(BankMsg::Send {
                from_address: state.owner.to_string(),
                to_address: state.liquidity.to_string(),
                amount: sendpremium,
                });
            //transfer premium in stblecoins from recipient to liq pool
        }
        else {
            let mut res = Response::new();
            res.add_message(BankMsg::Send {
                from_address: state.liquidity.to_string(),
                to_address: state.owner.to_string(),
                amount: sendpremium,
                });
            let collateral = coins(state.quantity.into() * state.strike_price.amount, state.conv_coin);
            res.add_message(BankMsg::Send {
                from_address: state.owner.to_string(),
                to_address: state.liquidity.to_string(),
                amount: collateral,
                });
            // transfer premium from liq pool to recipient && transfer collateral from recipient to liq pool (strike_price * quantity) 
        }
    }
    
    Ok(Response::new().add_attribute("method", "reset"))
}

pub fn try_expires(deps: DepsMut, info: MessageInfo, env: Env) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    
    let cur_price = 8.01_f64; // from price oracle

    if state.expiration_date > env.block.height {
        return Err(ContractError :: Unauthorized {});
    } 

    if state.putcall {
        if state.buysell && cur_price > state.strike_price.amount {
            let amt_owed = state.quantity as u128 * (cur_price as u128 - u128::from(state.strike_price.amount));
            // transfer amount in stble coins from liq pool to recipient
            let amt = coins(amt_owed, state.conv_coin);
            let mut res = Response::new();
            res.add_message(BankMsg::Send {
                from_address: state.liquidity.to_string(),
                to_address: state.owner.to_string(),
                amount: amt,
                });
        }
        if !state.buysell {
            if cur_price > state.strike_price.amount {
                // transfer (cur_price * quantity) in stblecoins from liq pool to recipient
                let amt = coins((state.quantity as f64 * cur_price) as u128, state.conv_coin);
                let mut res = Response::new();
                res.add_message(BankMsg::Send {
                    from_address: state.liquidity.to_string(),
                    to_address: state.owner.to_string(),
                    amount: amt,
                    });
            }
            else {
                // transfer quantity coins from liq pool to recipient
                let amt = coins(state.quantity as u128, state.opt_coin);
                let mut res = Response::new();
                res.add_message(BankMsg::Send {
                    from_address: state.liquidity.to_string(),
                    to_address: state.owner.to_string(),
                    amount: amt,
                    });
            }
        }
    }
    else {
        if state.buysell && cur_price < state.strike_price.amount {
            let amt_owed = state.quantity as f64 * (state.strike_price.amount - cur_price);
            let amt = coins(amt_owed as u128, state.conv_coin);
            let mut res = Response::new();
            res.add_message(BankMsg::Send {
                from_address: state.liquidity.to_string(),
                to_address: state.owner.to_string(),
                amount: amt,
                });
            // transfer amount from liq pool to recipient
        }
        if !state.buysell {
            if cur_price < state.strike_price.amount {
                let mut res = Response::new();
                res.add_message(BankMsg::Send {
                    from_address: state.liquidity.to_string(),
                    to_address: state.owner.to_string(),
                    amount: coins(state.quantity as u128, state.opt_coin),
                    });
                // transfer (quantity) coins from liq pool to recipient
            }
            else {
                let mut res = Response::new();
                res.add_message(BankMsg::Send {
                    from_address: state.liquidity.to_string(),
                    to_address: state.owner.to_string(),
                    amount: coins(state.strike_price.amount * state.quantity as u128, state.conv_coin),
                    });
                // transfer quantity * strike_price stblecoins from liq pool to recipient
            }
        }

    }

    STATE.remove(deps.storage);

    Ok(Response::new().add_attribute("method", "reset"))
}

pub fn try_selloption(deps: DepsMut, info: MessageInfo, env: Env, recipient: Addr) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    let cur_price = 8.01_f64; // from price oracle
    let vol = 1.00; // volatility from price oracle
    const RATE:f64 = 0.0021; // constant risk free rate of return
    
    let expire = state.expiration_date as f64;

    if state.expiration_date <= env.block.height {
        return Err(ContractError :: Unauthorized {});
    } 

    let amount = state.strike_price.len() as f64;
    
    let d1 = (((cur_price / amount).ln()) + expire * (RATE + ((vol * vol) / 2.0))) / (vol * expire.sqrt());
    let d2 = (((cur_price / amount).ln()) + expire * (RATE - ((vol * vol) / 2.0))) / (vol * expire.sqrt());

    let n = Uniform::new(0.0, 1.0).unwrap();

    if state.putcall {
        let call_price = (cur_price * n.cdf(d1)) - (amount * (-RATE * expire).exp() * n.cdf(d2));
        let totalpremium = call_price * (state.quantity as f64);
        if state.buysell {
            let mut res = Response::new();
                res.add_message(BankMsg::Send {
                    from_address: state.liquidity.to_string(),
                    to_address: state.owner.to_string(),
                    amount: coins(totalpremium, state.conv_coin),
                    });
            //transfer money from liq pool to recipient 
        }
        else {
            let mut res = Response::new();
                res.add_message(BankMsg::Send {
                    from_address: state.owner.to_string(),
                    to_address: state.liquidity.to_string(),
                    amount: coins(totalpremium, state.conv_coin),
                    });

                    res.add_message(BankMsg::Send {
                        from_address: state.liquidity.to_string(),
                        to_address: state.owner.to_string(),
                        amount: coins(state.quantity, state.opt_coin),
                        });
            // transfer money from liq pool to recipient
        }
    }
    else {
        let put_price = (amount * (-RATE * expire).exp() * n.cdf(-d2)) - (cur_price * n.cdf(-d1));
        let totalpremium = put_price * (state.quantity as f64);
        
        if state.buysell {
            let mut res = Response::new();
                res.add_message(BankMsg::Send {
                    from_address: state.liquidity.to_string(),
                    to_address: state.owner.to_string(),
                    amount: coins(totalpremium, state.conv_coin),
                    });
           
        }
        else {
            let mut res = Response::new();
                res.add_message(BankMsg::Send {
                    from_address: state.owner.to_string(),
                    to_address: state.liquidity.to_string(),
                    amount: coins(totalpremium, state.conv_coin),
                    });

            // transfer money from liq pool to recipient
        }
    }
    
    // delete option
    STATE.remove(deps.storage);

    Ok(Response::new().add_attribute("method", "reset"))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_count(deps)?),
    }
}

fn query_count(deps: Deps) -> StdResult<ConfigResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(state)
}

