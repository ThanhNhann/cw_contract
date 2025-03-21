use crate::state::{Ballot, Poll};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreatePoll {
        poll_id: String,
        question: String,
        options: Vec<String>,
    },
    Vote {
        poll_id: String,
        vote: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetAllPollsResponse)]
    GetAllPolls {},
    #[returns(GetPollResponse)]
    GetPoll { poll_id: String },
    #[returns(GetUserVoteResponse)]
    GetUserVote { user: Addr, poll_id: String },
}

#[cw_serde]
pub struct GetAllPollsResponse {
    pub polls: Vec<Poll>,
}

#[cw_serde]
pub struct GetPollResponse {
    pub poll: Option<Poll>,
}

#[cw_serde]
pub struct GetUserVoteResponse {
    pub vote: Option<Ballot>,
}
