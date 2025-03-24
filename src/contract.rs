#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, GetAllPollsResponse, GetPollResponse, GetUserVoteResponse, InstantiateMsg, QueryMsg,
};
use crate::state::{Ballot, Config, Poll, BALLOTS, CONFIG, POLLS};
use cosmwasm_std::Addr;

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
        fee: info.funds[0].clone(),
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", validated_admin.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreatePoll {
            poll_id,
            question,
            options,
        } => execute::execute_create_poll(deps, info, poll_id, question, options),
        ExecuteMsg::Vote { poll_id, vote } => execute::execute_vote(deps, info, poll_id, vote),
        ExecuteMsg::ClosePoll { poll_id } => execute::execute_close_poll(deps, info, poll_id),
    }
}

pub mod execute {

    use crate::state::BALLOTS;

    use super::*;

    pub fn execute_create_poll(
        deps: DepsMut,
        info: MessageInfo,
        poll_id: String,
        question: String,
        options: Vec<String>,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        if info.funds[0].denom != config.fee.denom || info.funds[0].amount < config.fee.amount {
            return Err(ContractError::InsufficientFunds {});
        }

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
            question: question.clone(),
            options: opts,
            is_active: true,
        };

        POLLS.save(deps.storage, &poll_id, &new_poll)?;

        Ok(Response::new()
            .add_attribute("action", "create_poll")
            .add_attribute("poll_id", poll_id)
            .add_attribute("creator", info.sender.to_string())
            .add_attribute("question", question)
            .add_attribute("options", options_clone.join(", "))
            .add_attribute("is_active", "true"))
    }

    pub fn execute_vote(
        deps: DepsMut,
        info: MessageInfo,
        poll_id: String,
        vote: String,
    ) -> Result<Response, ContractError> {
        let mut poll =
            POLLS
                .may_load(deps.storage, &poll_id)?
                .ok_or(ContractError::PollNotFound {
                    poll_id: poll_id.clone(),
                })?;

        if !poll.is_active {
            return Err(ContractError::PollClosed { poll_id });
        }

        BALLOTS.update(
            deps.storage,
            (info.sender.clone(), &poll_id),
            |ballot| -> StdResult<Ballot> {
                match ballot {
                    Some(ballot) => {
                        let old_position = poll
                            .options
                            .iter()
                            .position(|option| option.0 == ballot.option)
                            .unwrap();
                        poll.options[old_position].1 -= 1;
                        Ok(Ballot {
                            option: vote.clone(),
                        })
                    }
                    None => Ok(Ballot {
                        option: vote.clone(),
                    }),
                }
            },
        )?;

        let vote_position = poll
            .options
            .iter()
            .position(|(option, _)| option == &vote)
            .ok_or(ContractError::InvalidVote {})?;

        poll.options[vote_position].1 += 1;
        POLLS.save(deps.storage, &poll_id, &poll)?;

        Ok(Response::new()
            .add_attribute("action", "vote")
            .add_attribute("poll_id", poll_id)
            .add_attribute("voter", info.sender.to_string())
            .add_attribute("vote", vote))
    }

    pub fn execute_close_poll(
        deps: DepsMut,
        info: MessageInfo,
        poll_id: String,
    ) -> Result<Response, ContractError> {
        let mut poll =
            POLLS
                .may_load(deps.storage, &poll_id)?
                .ok_or(ContractError::PollNotFound {
                    poll_id: poll_id.clone(),
                })?;

        if info.sender != poll.creator && info.sender != CONFIG.load(deps.storage)?.admin {
            return Err(ContractError::Unauthorized {});
        }

        let config = CONFIG.load(deps.storage)?;
        poll.is_active = false;
        POLLS.save(deps.storage, &poll_id, &poll)?;

        let bank_msg = BankMsg::Send {
            to_address: poll.creator.to_string(),
            amount: vec![config.fee],
        };

        Ok(Response::new()
            .add_message(bank_msg)
            .add_attribute("action", "close_poll")
            .add_attribute("poll_id", poll_id))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAllPolls {} => to_json_binary(&query::get_all_polls(deps)?),
        QueryMsg::GetPoll { poll_id } => to_json_binary(&query::get_poll(deps, poll_id)?),
        QueryMsg::GetUserVote { poll_id, user } => {
            to_json_binary(&query::get_user_vote(deps, poll_id, user)?)
        }
    }
}

pub mod query {
    use super::*;

    pub fn get_all_polls(deps: Deps) -> StdResult<GetAllPollsResponse> {
        let polls: Vec<Poll> = POLLS
            .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?
            .into_iter()
            .map(|(_, poll)| poll)
            .collect();
        Ok(GetAllPollsResponse { polls })
    }

    pub fn get_poll(deps: Deps, poll_id: String) -> StdResult<GetPollResponse> {
        let poll = POLLS.load(deps.storage, &poll_id)?;
        Ok(GetPollResponse { poll: Some(poll) })
    }

    pub fn get_user_vote(
        deps: Deps,
        poll_id: String,
        user: Addr,
    ) -> StdResult<GetUserVoteResponse> {
        let vote = BALLOTS.load(deps.storage, (user, &poll_id)).ok();
        Ok(GetUserVoteResponse { vote })
    }
}

#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate};
    use crate::error::ContractError;
    use crate::msg::{ExecuteMsg, InstantiateMsg};
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{attr, from_json, Addr, Coin, MessageInfo, Uint128};
    // use crate::state::{POLLS};

    use super::*;

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = deps.api.addr_make("sender").to_string();
        let admin = deps.api.addr_make("admin").to_string();
        let info = MessageInfo {
            sender: Addr::unchecked(sender.clone()),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::from(1000u128),
            }],
        };

        // Test with no admin specified (should use sender as admin)
        let msg = InstantiateMsg { admin: None };
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![attr("method", "instantiate"), attr("admin", sender)]
        );

        // Test with specific admin
        let msg = InstantiateMsg {
            admin: Some(admin.clone()),
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![attr("method", "instantiate"), attr("admin", admin)]
        );
    }

    #[test]
    fn test_execute_create_poll() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = deps.api.addr_make("sender").to_string();
        let info = MessageInfo {
            sender: Addr::unchecked(sender.clone()),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::from(1000u128),
            }],
        };

        let insufficient_funds_info = MessageInfo {
            sender: Addr::unchecked(sender.clone()),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::from(100u128),
            }],
        };

        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let valid_options = [("Option 1", 0), ("Option 2", 0), ("Option 3", 0)];
        let invalid_options = [
            ("Option 1", 0),
            ("Option 2", 0),
            ("Option 3", 0),
            ("Option 4", 0),
            ("Option 5", 0),
            ("Option 6", 0),
            ("Option 7", 0),
            ("Option 8", 0),
            ("Option 9", 0),
            ("Option 10", 0),
            ("Option 11", 0),
        ];

        let question = "What is the best color?";
        let poll_id = "poll1";

        // create poll with invalid length of options
        let create_poll_msg_invalid_len_options = ExecuteMsg::CreatePoll {
            poll_id: poll_id.to_string(),
            question: question.to_string(),
            options: invalid_options
                .iter()
                .map(|(option, _)| option.to_string())
                .collect(),
        };

        let res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            create_poll_msg_invalid_len_options,
        )
        .unwrap_err();
        assert_eq!(res, ContractError::TooManyOptions {});

        // create poll with insufficient funds
        let create_poll_msg_invalid_len_options = ExecuteMsg::CreatePoll {
            poll_id: poll_id.to_string(),
            question: question.to_string(),
            options: valid_options
                .iter()
                .map(|(option, _)| option.to_string())
                .collect(),
        };

        let res = execute(
            deps.as_mut(),
            env.clone(),
            insufficient_funds_info,
            create_poll_msg_invalid_len_options,
        )
        .unwrap_err();
        assert_eq!(res, ContractError::InsufficientFunds {});

        // create poll with valid
        let create_poll_msg = ExecuteMsg::CreatePoll {
            poll_id: poll_id.to_string(),
            question: question.to_string(),
            options: valid_options
                .iter()
                .map(|(option, _)| option.to_string())
                .collect(),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), create_poll_msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "create_poll"),
                attr("poll_id", poll_id),
                attr("creator", sender),
                attr("question", question),
                attr("options", "Option 1, Option 2, Option 3"),
                attr("is_active", "true")
            ]
        );
    }

    #[test]
    fn test_execute_vote_invalid() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = deps.api.addr_make("sender").to_string();
        let info = MessageInfo {
            sender: Addr::unchecked(sender.clone()),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::from(1000u128),
            }],
        };

        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let options = [("Option 1", 0), ("Option 2", 0), ("Option 3", 0)];
        let question = "What is the best color?";
        let poll_id = "poll1";

        let create_poll_msg = ExecuteMsg::CreatePoll {
            poll_id: poll_id.to_string(),
            question: question.to_string(),
            options: options
                .iter()
                .map(|(option, _)| option.to_string())
                .collect(),
        };

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), create_poll_msg).unwrap();

        let invalid_vote_msg = ExecuteMsg::Vote {
            poll_id: "poll2".to_string(),
            vote: "Option 4".to_string(),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), invalid_vote_msg).unwrap_err();
        assert_eq!(
            res,
            ContractError::PollNotFound {
                poll_id: "poll2".to_string()
            }
        );

        let invalid_vote2_msg = ExecuteMsg::Vote {
            poll_id: "poll1".to_string(),
            vote: "Option 4".to_string(),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), invalid_vote2_msg).unwrap_err();
        assert_eq!(res, ContractError::InvalidVote {});
    }

    #[test]
    fn test_execute_vote_valid() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = deps.api.addr_make("sender").to_string();
        let info = MessageInfo {
            sender: Addr::unchecked(sender.clone()),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::from(1000u128),
            }],
        };

        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let options = [("Option 1", 0), ("Option 2", 0), ("Option 3", 0)];
        let question = "What is the best color?";
        let poll_id = "poll1";

        let create_poll_msg = ExecuteMsg::CreatePoll {
            poll_id: poll_id.to_string(),
            question: question.to_string(),
            options: options
                .iter()
                .map(|(option, _)| option.to_string())
                .collect(),
        };

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), create_poll_msg).unwrap();

        let vote_msg = ExecuteMsg::Vote {
            poll_id: poll_id.to_string(),
            vote: "Option 1".to_string(),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), vote_msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "vote"),
                attr("poll_id", poll_id),
                attr("voter", sender),
                attr("vote", "Option 1")
            ]
        );

        // Verify the vote was counted
        let poll = POLLS.load(deps.as_ref().storage, poll_id).unwrap();
        assert_eq!(poll.options[0].1, 1); // Option 1 should have 1 vote
        assert_eq!(poll.options[1].1, 0); // Option 2 should have 0 votes
        assert_eq!(poll.options[2].1, 0); // Option 3 should have 0 votes
    }

    #[test]
    fn test_query_get_all_polls() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = deps.api.addr_make("sender").to_string();
        let info = MessageInfo {
            sender: Addr::unchecked(sender.clone()),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::from(1000u128),
            }],
        };

        // Test with no poll was created
        let query_msg = QueryMsg::GetAllPolls {};
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let all_polls: GetAllPollsResponse = from_json(&res).unwrap();
        assert_eq!(all_polls.polls.len(), 0);

        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let create_poll_msg = ExecuteMsg::CreatePoll {
            poll_id: "poll1".to_string(),
            question: "What is the best color?".to_string(),
            options: vec![
                "Option 1".to_string(),
                "Option 2".to_string(),
                "Option 3".to_string(),
            ],
        };

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), create_poll_msg).unwrap();

        let query_msg = QueryMsg::GetAllPolls {};
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let all_polls: GetAllPollsResponse = from_json(&res).unwrap();
        assert_eq!(all_polls.polls.len(), 1);
    }

    #[test]
    fn test_query_get_poll() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = deps.api.addr_make("sender").to_string();
        let info = MessageInfo {
            sender: Addr::unchecked(sender.clone()),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::from(1000u128),
            }],
        };

        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let create_poll_msg = ExecuteMsg::CreatePoll {
            poll_id: "poll1".to_string(),
            question: "What is the best color?".to_string(),
            options: vec![
                "Option 1".to_string(),
                "Option 2".to_string(),
                "Option 3".to_string(),
            ],
        };

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), create_poll_msg).unwrap();

        let query_msg = QueryMsg::GetPoll {
            poll_id: "poll1".to_string(),
        };
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let poll: GetPollResponse = from_json(&res).unwrap();
        assert_eq!(
            poll.clone().poll.unwrap().question,
            "What is the best color?"
        );
        assert_eq!(poll.clone().poll.unwrap().options.len(), 3);
        assert_eq!(poll.clone().poll.unwrap().options[0].0, "Option 1");
        assert_eq!(poll.clone().poll.unwrap().options[1].0, "Option 2");
        assert_eq!(poll.clone().poll.unwrap().options[2].0, "Option 3");
    }

    #[test]
    fn test_query_get_user_vote() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = deps.api.addr_make("sender").to_string();
        let info = MessageInfo {
            sender: Addr::unchecked(sender.clone()),
            funds: vec![Coin {
                denom: "uatom".to_string(),
                amount: Uint128::from(1000u128),
            }],
        };

        let msg = InstantiateMsg { admin: None };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let create_poll_msg = ExecuteMsg::CreatePoll {
            poll_id: "poll1".to_string(),
            question: "What is the best color?".to_string(),
            options: vec![
                "Option 1".to_string(),
                "Option 2".to_string(),
                "Option 3".to_string(),
            ],
        };

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), create_poll_msg).unwrap();

        let vote_msg = ExecuteMsg::Vote {
            poll_id: "poll1".to_string(),
            vote: "Option 1".to_string(),
        };

        let _res = execute(deps.as_mut(), env.clone(), info.clone(), vote_msg).unwrap();

        let query_msg = QueryMsg::GetUserVote {
            poll_id: "poll1".to_string(),
            user: Addr::unchecked(sender.clone()),
        };

        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let user_vote: GetUserVoteResponse = from_json(&res).unwrap();
        assert_eq!(user_vote.vote.unwrap().option, "Option 1");

        let query_msg2 = QueryMsg::GetUserVote {
            poll_id: "poll1".to_string(),
            user: Addr::unchecked("other_user".to_string()),
        };

        let res2 = query(deps.as_ref(), env.clone(), query_msg2).unwrap();
        let user_vote2: GetUserVoteResponse = from_json(&res2).unwrap();
        assert!(user_vote2.vote.is_none());
    }

    #[test]
    fn test_execute_close_poll() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = deps.api.addr_make("sender").to_string();
        let admin = deps.api.addr_make("admin").to_string();
        let fee = Coin {
            denom: "uatom".to_string(),
            amount: Uint128::from(1000u128),
        };
        let info = MessageInfo {
            sender: Addr::unchecked(sender.clone()),
            funds: vec![fee.clone()],
        };

        let unauthorized_info = MessageInfo {
            sender: Addr::unchecked("unauthorized".to_string()),
            funds: vec![],
        };

        let msg = InstantiateMsg {
            admin: Some(admin.clone()),
        };
        let _res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // create poll 1
        let create_poll1_msg = ExecuteMsg::CreatePoll {
            poll_id: "poll1".to_string(),
            question: "What is the best color?".to_string(),
            options: vec![
                "Option 1".to_string(),
                "Option 2".to_string(),
                "Option 3".to_string(),
            ],
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), create_poll1_msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "create_poll"),
                attr("poll_id", "poll1"),
                attr("creator", &sender),
                attr("question", "What is the best color?"),
                attr("options", "Option 1, Option 2, Option 3"),
                attr("is_active", "true")
            ]
        );

        // create poll 2
        let create_poll2_msg = ExecuteMsg::CreatePoll {
            poll_id: "poll2".to_string(),
            question: "What is the best color?".to_string(),
            options: vec![
                "Option 1".to_string(),
                "Option 2".to_string(),
                "Option 3".to_string(),
            ],
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), create_poll2_msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "create_poll"),
                attr("poll_id", "poll2"),
                attr("creator", &sender),
                attr("question", "What is the best color?"),
                attr("options", "Option 1, Option 2, Option 3"),
                attr("is_active", "true")
            ]
        );

        // query all polls
        let query_msg = QueryMsg::GetAllPolls {};
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let all_polls: GetAllPollsResponse = from_json(&res).unwrap();
        assert_eq!(all_polls.polls.len(), 2);

        // testing close polls
        let close_poll_msg = ExecuteMsg::ClosePoll {
            poll_id: "poll1".to_string(),
        };

        // close poll with an unauthorized user
        let res = execute(
            deps.as_mut(),
            env.clone(),
            unauthorized_info.clone(),
            close_poll_msg.clone(),
        )
        .unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});

        // close poll1 with the creator
        let res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            close_poll_msg.clone(),
        )
        .unwrap();
        assert_eq!(
            res.attributes,
            vec![attr("action", "close_poll"), attr("poll_id", "poll1")]
        );

        // Check that the bank message was added to return the fee
        assert_eq!(res.messages.len(), 1);
        match &res.messages[0].msg {
            cosmwasm_std::CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address, &sender);
                assert_eq!(amount, &vec![fee.clone()]);
            }
            _ => panic!("Expected BankMsg::Send"),
        }

        let query_msg = QueryMsg::GetPoll {
            poll_id: "poll1".to_string(),
        };
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let poll: GetPollResponse = from_json(&res).unwrap();
        assert!(!poll.poll.unwrap().is_active);

        // close poll2 with the admin
        let close_poll2_msg = ExecuteMsg::ClosePoll {
            poll_id: "poll2".to_string(),
        };
        let res = execute(
            deps.as_mut(),
            env.clone(),
            MessageInfo {
                sender: Addr::unchecked(admin.clone()),
                funds: vec![],
            },
            close_poll2_msg,
        )
        .unwrap();
        assert_eq!(
            res.attributes,
            vec![attr("action", "close_poll"), attr("poll_id", "poll2")]
        );

        // Check that the bank message was added to return the fee
        assert_eq!(res.messages.len(), 1);
        match &res.messages[0].msg {
            cosmwasm_std::CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address, &sender);
                assert_eq!(amount, &vec![fee.clone()]);
            }
            _ => panic!("Expected BankMsg::Send"),
        }

        let query_msg = QueryMsg::GetPoll {
            poll_id: "poll2".to_string(),
        };
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let poll: GetPollResponse = from_json(&res).unwrap();
        assert!(!poll.poll.unwrap().is_active);

        let vote_msg = ExecuteMsg::Vote {
            poll_id: "poll1".to_string(),
            vote: "Option 1".to_string(),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), vote_msg).unwrap_err();
        assert_eq!(
            res,
            ContractError::PollClosed {
                poll_id: "poll1".to_string()
            }
        );
    }
}
