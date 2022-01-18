use std::cmp::Ordering;

use crate::msg::{CreatorResponse, HandleMsg, InitMsg, PotResponse, QueryMsg, SpinResponse};
use crate::rand::{new_entropy, sha_256};
use crate::state::{config, config_read, State};
use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Coin, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage, Uint128,
};
use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let owner = msg.creator.unwrap_or(env.message.sender.clone());
    let owner = deps.api.canonical_address(&owner)?;

    let prng_seed: Vec<u8> = sha_256(base64::encode(&msg.entropy).as_bytes()).to_vec();

    let state = State {
        pot: 0,
        owner,
        prng_seed,
        current_round: 0,
    };

    config(&mut deps.storage).save(&state)?;

    debug_print!("Contract was initialized by {}", env.message.sender);

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Spin {} => handle_spin(deps, env),
        HandleMsg::CashOut { quantity } => handle_cash_out(deps, env, quantity),
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Pot {} => to_binary(&query_pot(deps)),
        QueryMsg::Creator {} => to_binary(&query_creator(deps)),
    }
}

pub fn handle_spin<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let state = config(&mut deps.storage).load()?;

    let mut total_coins_sent = Uint128::zero();
    for coin in env.message.sent_funds.iter() {
        if coin.denom != "uscrt" {
            return Err(StdError::generic_err(
                "Only uscrt is supported. Invalid token sent. ",
            ));
        }
        total_coins_sent += coin.amount;
    }
    if total_coins_sent.is_zero() {
        return Err(StdError::generic_err("No coins sent"));
    }

    let predicted_winnings = total_coins_sent.u128() * 2;
    if predicted_winnings > state.pot.into() {
        return Err(StdError::generic_err("Not enough funds in pot"));
    }

    let entropies = vec![
        env.block.height.to_be_bytes().clone(),
        env.block.time.to_be_bytes().clone(),
        state.pot.to_be_bytes().clone(),
    ];
    let rand_seed = new_entropy(&entropies, &state.prng_seed);
    let mut rng = ChaChaRng::from_seed(rand_seed);

    let rand_num = (rng.next_u32() % 6) as u8;

    let result = match state.current_round.cmp(&rand_num) {
        Ordering::Equal => "win",
        _ => "lose",
    };

    println!("rand_num: {}", rand_num);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&SpinResponse {
            result: "".to_string(),
            winnings: None,
        })?),
    })
}
pub fn handle_cash_out<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _quantity: Option<Vec<Coin>>,
) -> StdResult<HandleResponse> {
    Ok(HandleResponse::default())
}

pub fn query_pot<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<PotResponse> {
    let state = config_read(&deps.storage).load()?;

    Ok(PotResponse {
        quantity: state.pot,
    })
}
pub fn query_creator<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<CreatorResponse> {
    let state = config_read(&deps.storage).load()?;
    let creator = deps.api.human_address(&state.owner)?;
    Ok(CreatorResponse { creator })
}
#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, BlockInfo};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            creator: None,
            end_height: None,
            end_time: None,
            entropy: "awdada".to_string(),
        };
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_spin() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            creator: None,
            end_height: None,
            end_time: None,
            entropy: "awdadae".to_string(),
        };
        let mut env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let _res = init(&mut deps, env.clone(), msg).unwrap();

        let msg = HandleMsg::Spin {};

        let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
        env.block = BlockInfo {
            chain_id: "test-chain-id".to_string(),
            height: 1,
            time: 1,
        };
        let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    }
}
