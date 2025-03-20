#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_json_binary};
use cw2::set_contract_version;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, Poll, Ballot ,CONFIG, POLLS};


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw_contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = msg.admin.unwrap_or(info.sender.to_string());
    let validated_admin = deps.api.addr_validate(&admin)?;

    let config = Config {
        admin: validated_admin.clone(),
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", validated_admin.to_string())
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreatePoll { poll_id, question, options } => execute::execute_create_poll(deps, info, poll_id, question, options),
        ExecuteMsg::Vote { poll_id,  vote } => execute::execute_vote(deps, info, poll_id, vote),
    }
}

pub mod execute {

    use crate::state::BALLOTS;

    use super::*;

    pub fn execute_create_poll(deps: DepsMut, info: MessageInfo,  poll_id: String, question: String, options: Vec<String>) -> Result<Response, ContractError> {
        if options.len() > 10 {
            return Err(ContractError::TooManyOptions {});
        }

        let mut opts: Vec<(String, u64)> = Vec::new();
        let options_clone = options.clone();

        for option in options {
            opts.push((option, 0));
        }

        let new_poll = Poll {
            creator: info.sender.clone(),
            question:  question.clone(),
            options: opts,
        };

        POLLS.save(deps.storage, poll_id.clone(), &new_poll)?;

        Ok(Response::new()
            .add_attribute("action", "create_poll")
            .add_attribute("poll_id", poll_id)
            .add_attribute("creator", info.sender.to_string())
            .add_attribute("question", question)
            .add_attribute("options", options_clone.join(", "))
        )
    }

    pub fn execute_vote(deps: DepsMut, info: MessageInfo, poll_id: String, vote: String) -> Result<Response, ContractError> {
        let mut poll = POLLS.may_load(deps.storage, poll_id.clone())?
            .ok_or(ContractError::PollNotFound { poll_id: poll_id.clone() })?;

        BALLOTS.update(deps.storage, (info.sender.clone(), poll_id.clone()), |ballot| -> StdResult<Ballot> {
            match ballot {
                Some(ballot) => {
                    let old_position = poll.options.iter().position(|option| option.0 == ballot.option).unwrap();
                    poll.options[old_position].1 -= 1;
                    Ok(Ballot { option: vote.clone() })
                }
                None => {
                    Ok(Ballot { option: vote.clone() })
                }
            }
        })?;
        
        if let Some((_, count)) = poll.options.iter_mut().find(|(option, _)| option == &vote) {
            *count += 1;
            POLLS.save(deps.storage, poll_id.clone(), &poll)?;
        }
        Ok(Response::new()
            .add_attribute("action", "vote")
            .add_attribute("poll_id", poll_id)
            .add_attribute("voter", info.sender.to_string())
            .add_attribute("vote", vote)
        )
    }
}

// #[cfg_attr(not(feature = "library"), entry_point)]
// pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
//     match msg {
//         QueryMsg::GetCount {} => to_json_binary(&query::get_count(_deps)?),
//     }
// }

// pub mod query {
//     use crate::msg::GetCountResponse;

//     use super::*;

//     pub fn get_count(deps: Deps) -> StdResult<GetCountResponse> {
//         let state = STATE.load(deps.storage)?;
//         Ok(GetCountResponse { count: state.count })
//     }
// }
#[cfg(test)]
mod tests {
    use cosmwasm_std::{attr, MessageInfo, Addr};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use crate::contract::instantiate;
    use crate::msg::InstantiateMsg;

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = deps.api.addr_make("sender").to_string();
        let admin = deps.api.addr_make("admin").to_string();
        let info = MessageInfo {
            sender: Addr::unchecked(sender.clone()),
            funds: vec![],
        };

        // Test with no admin specified (should use sender as admin)
        let msg = InstantiateMsg { admin: None };
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![attr("method", "instantiate"), attr("admin", sender)]
        );

        // Test with specific admin
        let msg = InstantiateMsg { admin: Some(admin.clone()) };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![attr("method", "instantiate"), attr("admin", admin)]
        );
    }
    
}
