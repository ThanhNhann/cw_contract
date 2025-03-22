use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Too many options")]
    TooManyOptions {},

    #[error("Poll not found: {poll_id}")]
    PollNotFound { poll_id: String },

    #[error("Invalid vote option")]
    InvalidVote {},

    #[error("Insufficient funds")]
    InsufficientFunds {},
}
