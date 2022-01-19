use std::cmp::Ordering;

use crate::msg::{CreatorResponse, HandleMsg, InitMsg, PotResponse, QueryMsg, SpinResponse};
use crate::rand::{new_entropy, sha_256};
use crate::state::{config, config_read, State};
use cosmwasm_std::{
    debug_print, log, to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Env, Extern,
    HandleResponse, InitResponse, Querier, StdError, StdResult, Storage, Uint128,
};
use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let owner = msg.creator.unwrap_or_else(|| env.message.sender.clone());
    let owner = deps.api.canonical_address(&owner)?;

    let prng_seed: Vec<u8> = sha_256(base64::encode(&msg.entropy).as_bytes()).to_vec();

    let mut total_coins_sent = Uint128::zero();
    for coin in env.message.sent_funds.iter() {
        if coin.denom != "uscrt" {
            return Err(StdError::generic_err(
                "Only uscrt is supported. Invalid token sent. ",
            ));
        }
        total_coins_sent += coin.amount;
    }

    let state = State {
        pot: total_coins_sent,
        owner,
        prng_seed,
        current_round: 0,
    };

    config(&mut deps.storage).save(&state)?;

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
    let mut state = config(&mut deps.storage).load()?;

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

    //list of entropies to be used for the spin RNG
    let entropies = vec![env.block.height.to_be_bytes(), env.block.time.to_be_bytes()];
    //construct a new entropy that combines multiple inputs including preset one from the creator
    let rand_seed = new_entropy(&entropies, &state.prng_seed);
    let mut rng = ChaChaRng::from_seed(rand_seed);
    let rand_num = (rng.next_u32() % 6) as u8;

    let (result, message): (SpinResponse, Option<BankMsg>) =
        match state.current_round.cmp(&rand_num) {
            //the player has lost if the spinned number is the same as his round number
            Ordering::Equal => {
                state.pot += total_coins_sent;
                state.current_round = 0;

                let spin_res = SpinResponse {
                    result: String::from("You just died. You lost all your money."),
                    winnings: None,
                };

                (spin_res, None)
            }
            _ => {
                //this error cannot happen because of the inital check with predicted_winnings, but it is here for completeness
                state.pot =
                    (state.pot - total_coins_sent).expect("Critical Error: Pot is negative");
                //since the spin happens every round, there is a possibility that no player will lose, thus we reset after 6 rounds.
                state.current_round = (state.current_round + 1) % 6;

                let spin_res = SpinResponse {
                    result: String::from("Congrats! You just won some money."),
                    winnings: Some(Uint128::from(predicted_winnings)),
                };

                let send_msg = BankMsg::Send {
                    from_address: env.contract.address,
                    to_address: env.message.sender,
                    amount: vec![Coin {
                        denom: "uscrt".to_string(),
                        amount: Uint128::from(predicted_winnings),
                    }],
                };

                (spin_res, Some(send_msg))
            }
        };
    config(&mut deps.storage).save(&state)?;

    Ok(HandleResponse {
        messages: message.map_or_else(Vec::new, |msg| vec![CosmosMsg::Bank(msg)]),
        log: vec![
            log("predicted_win", predicted_winnings.to_string()),
            log("generated_value", rand_num.to_string()),
            log("pot", state.pot.to_string()),
            log("current_round", state.current_round.to_string()),
        ],
        data: Some(to_binary(&result)?),
    })
}
pub fn handle_cash_out<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    quantity: Option<Uint128>,
) -> StdResult<HandleResponse> {
    let mut state = config(&mut deps.storage).load()?;
    state.pot = (state.pot - quantity.unwrap_or_else(|| state.pot))
        .expect("Error! Can't withdraw more than the pot has");

    let send_msg = BankMsg::Send {
        from_address: env.contract.address,
        to_address: env.message.sender,
        amount: vec![Coin {
            denom: "uscrt".to_string(),
            amount: quantity.unwrap_or_else(|| state.pot),
        }],
    };

    config(&mut deps.storage).save(&state)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Bank(send_msg)],
        log: vec![log("pot", state.pot.to_string())],
        data: None,
    })
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
    use cosmwasm_std::coins;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            creator: None,
            end_height: None,
            end_time: None,
            entropy: "awdada".to_string(),
        };
        let env = mock_env("creator", &coins(1000, "uscrt"));

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
        let env = mock_env("creator", &coins(1000, "uscrt"));
        let env_player = mock_env("player", &coins(2, "uscrt"));

        // we can just call .unwrap() to assert this was a success
        let _res = init(&mut deps, env.clone(), msg).unwrap();

        let msg = HandleMsg::Spin {};

        let _res = handle(&mut deps, env_player, msg.clone()).unwrap();
    }
}
