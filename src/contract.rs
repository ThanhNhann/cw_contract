#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_json_binary};
use cw2::set_contract_version;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};


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
    let state = State {
        count: msg.count,
        owner: info.sender.clone(),
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("count", msg.count.to_string())
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
        ExecuteMsg::Increment {} => execute::increment(deps),
        ExecuteMsg::Reset { count } => execute::reset(deps, info, count),
    }
}

pub mod execute {

    use super::*;

    pub fn increment(deps: DepsMut) -> Result<Response, ContractError> {
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.count += 1;
            Ok(state)
        })?;

        Ok(Response::new().add_attribute("action", "increment"))
    }

    pub fn reset(deps: DepsMut, info: MessageInfo, count: i32) -> Result<Response, ContractError> {
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            if info.sender != state.owner {
                return Err(ContractError::Unauthorized {});
            }
            state.count = count;
            Ok(state)
        })?;

        Ok(Response::new()
            .add_attribute("action", "reset")
            .add_attribute("count", count.to_string())
        )
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_json_binary(&query::get_count(_deps)?),
    }
}

pub mod query {
    use crate::msg::GetCountResponse;

    use super::*;

    pub fn get_count(deps: Deps) -> StdResult<GetCountResponse> {
        let state = STATE.load(deps.storage)?;
        Ok(GetCountResponse { count: state.count })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{from_json, MessageInfo, Addr};
    use crate::msg::GetCountResponse;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { count: 17 };
        let info = MessageInfo {
            sender: Addr::unchecked("creator"),
            funds: vec![],
        };
        
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let state = query::get_count(deps.as_ref()).unwrap();
        assert_eq!(17, state.count);
    }
    

    #[test]
    fn increment() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { count: 17 };
        let info = MessageInfo {
            sender: Addr::unchecked("creator"),
            funds: vec![],
        };
        
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::Increment {};
        let info = MessageInfo {
            sender: Addr::unchecked("anyone"),
            funds: vec![],
        };
        
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        // assert_eq!(1, res.messages.len());

        let res_query = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: GetCountResponse = from_json(res_query).unwrap();
        assert_eq!(18, value.count);
    }
    

    #[test]
    fn reset_without_permission() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { count: 17 };
        let info = MessageInfo {
            sender: Addr::unchecked("creator"),
            funds: vec![],
        };
        
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::Reset { count: 0 };
        let info = MessageInfo {
            sender: Addr::unchecked("not_creator"),
            funds: vec![],
        };
        
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(ContractError::Unauthorized {}, res);
    }

    #[test]
    fn reset_with_permission() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { count: 17 };
        let info = MessageInfo {
            sender: Addr::unchecked("creator"),
            funds: vec![],
        };
        
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::Reset { count: 0 };
        let info = MessageInfo {
            sender: Addr::unchecked("creator"),
            funds: vec![],
        };
        
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res_query = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: GetCountResponse = from_json(res_query).unwrap();
        assert_eq!(0, value.count);
    }

}
