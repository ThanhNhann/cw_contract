# Poll Smart Contract

A CosmWasm smart contract for creating and managing polls on the Cosmos blockchain. This contract allows users to create polls, vote on them, and close them when finished.

## Features

- Create polls with multiple options (up to 10)
- Vote on active polls
- Close polls (by creator or admin)
- Query poll information and user votes
- Fee-based poll creation (fee is returned when poll is closed)
- Admin controls for poll management

## Contract Messages

### Instantiate
```rust
pub struct InstantiateMsg {
    pub admin: Option<String>,
}
```
- Initializes the contract with an optional admin address
- If no admin is specified, the sender becomes the admin
- Sets the fee for poll creation

### Execute Messages

#### CreatePoll
```rust
pub struct CreatePoll {
    pub poll_id: String,
    pub question: String,
    pub options: Vec<String>,
}
```
- Creates a new poll with the specified question and options
- Requires payment of the configured fee
- Maximum of 10 options allowed

#### Vote
```rust
pub struct Vote {
    pub poll_id: String,
    pub vote: String,
}
```
- Allows users to vote on an active poll
- Users can change their vote
- Votes are only allowed on active polls

#### ClosePoll
```rust
pub struct ClosePoll {
    pub poll_id: String,
}
```
- Closes a poll (can only be done by creator or admin)
- Returns the creation fee to the poll creator
- Prevents further voting on the poll

### Query Messages

#### GetAllPolls
```rust
pub struct GetAllPolls {}
```
- Returns a list of all polls in the contract

#### GetPoll
```rust
pub struct GetPoll {
    pub poll_id: String,
}
```
- Returns details of a specific poll

#### GetUserVote
```rust
pub struct GetUserVote {
    pub poll_id: String,
    pub user: String,
}
```
- Returns a user's vote for a specific poll

## State

### Config
```rust
pub struct Config {
    pub admin: Addr,
    pub fee: Coin,
}
```
- Stores admin address and poll creation fee

### Poll
```rust
pub struct Poll {
    pub creator: Addr,
    pub question: String,
    pub options: Vec<(String, u64)>,
    pub is_active: bool,
}
```
- Stores poll information including creator, question, options with vote counts, and active status

### Ballot
```rust
pub struct Ballot {
    pub option: String,
}
```
- Stores user votes for each poll

## Fee Structure

The contract implements a fee-based system for poll creation:

1. **Initial Fee**: When creating a poll, the creator must pay a fee specified during contract instantiation
   - The fee is stored in the contract's config
   - The fee amount and denomination are set during contract instantiation
   - Example: 1 ATOM (1000000 uatom) per poll creation

2. **Fee Return**: The fee is returned to the poll creator when the poll is closed
   - The fee is returned using a `BankMsg::Send` message
   - The fee is returned in its original denomination
   - The return happens automatically when the poll is closed by either the creator or admin

3. **Fee Purpose**: The fee serves as a deposit to:
   - Prevent spam creation of polls
   - Ensure poll creators have a stake in their polls
   - Encourage proper poll management (closing when finished)

## Implementation Details

### Storage
- Uses CosmWasm's storage system with the following maps:
  - `POLLS`: Maps poll_id to Poll struct
  - `BALLOTS`: Maps (user, poll_id) to Ballot struct
  - `CONFIG`: Stores contract configuration

### Vote Counting
- Votes are tracked using a tuple of (option, count) in the Poll struct
- When a user changes their vote:
  1. The old vote count is decremented
  2. The new vote count is incremented
  3. The user's ballot is updated

### Authorization
- Poll creation: Any user with sufficient funds
- Poll closure: Only the creator or admin
- Voting: Any user on active polls

### Error Handling
- Comprehensive error types for all failure cases
- Proper validation of inputs and state
- Clear error messages for debugging

## Usage Examples

### Creating a Poll
```rust
// Example using CosmJS
const createPollMsg = {
  create_poll: {
    poll_id: "poll1",
    question: "What is your favorite color?",
    options: ["Red", "Blue", "Green"]
  }
};

// Send with fee
const fee = {
  amount: "1000000", // 1 ATOM
  gas: "200000"
};

await client.execute(contractAddress, createPollMsg, fee);
```

### Voting on a Poll
```rust
const voteMsg = {
  vote: {
    poll_id: "poll1",
    vote: "Blue"
  }
};

await client.execute(contractAddress, voteMsg);
```

### Closing a Poll
```rust
const closePollMsg = {
  close_poll: {
    poll_id: "poll1"
  }
};

await client.execute(contractAddress, closePollMsg);
```

### Querying Poll Information
```rust
// Get all polls
const allPolls = await client.queryContractSmart(contractAddress, {
  get_all_polls: {}
});

// Get specific poll
const poll = await client.queryContractSmart(contractAddress, {
  get_poll: {
    poll_id: "poll1"
  }
});

// Get user's vote
const userVote = await client.queryContractSmart(contractAddress, {
  get_user_vote: {
    poll_id: "poll1",
    user: "user_address"
  }
});
```

## Deployment

### Prerequisites
1. Rust toolchain (latest stable)
2. CosmWasm CLI tools
3. Access to a Cosmos SDK chain with CosmWasm support

### Build
```bash
# Build the contract
cargo wasm

# Run tests
cargo test

# Generate schema
cargo schema
```

### Deploy
```bash
# Upload the contract
RES=$(wasmd tx wasm store artifacts/cw_contract.wasm --from wallet $TXFLAG -y --output json -b block)

# Get the code ID
CODE_ID=$(echo $RES | jq -r '.logs[0].events[-1].attributes[0].value')

# Instantiate the contract
INIT='{"admin": "cosmos1...", "fee": {"amount": "1000000", "denom": "uatom"}}'
wasmd tx wasm instantiate $CODE_ID "$INIT" --from wallet $TXFLAG -y
```

### Verify
```bash
# Query contract info
wasmd query wasm contract $CONTRACT_ADDRESS

# Query contract state
wasmd query wasm contract-state smart $CONTRACT_ADDRESS '{"get_all_polls":{}}'
```

## Testing

The contract includes comprehensive tests covering:
- Contract instantiation
- Poll creation with various scenarios
- Voting functionality
- Poll closing and fee return
- Query functionality
- Error cases and edge conditions

Run tests with:
```bash
cargo test
```

## Error Handling

The contract handles various error cases:
- Insufficient funds for poll creation
- Too many options in a poll
- Invalid votes
- Unauthorized poll closure
- Voting on closed polls
- Non-existent polls
