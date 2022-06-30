use crate::hub::{Batch, BooleanKey, InstantiateMsg, UnbondRequest};
use std::vec;

use apollo_protocol::utils::parse_contract_addr_from_instantiate_event;
use cosmwasm_std::{
    to_binary, Addr, BalanceResponse, BankMsg, BankQuery, Coin, CosmosMsg, DepsMut, Env,
    MessageInfo, QuerierWrapper, QueryRequest, Reply, Response, StdError, StdResult, SubMsg,
    SubMsgResponse, Uint128, WasmMsg, WasmQuery,
};
use cw20_base::msg::{ExecuteMsg as Cw20ExecuteMsg, QueryMsg as Cw20QueryMsg};
use cw_storage_plus::{Index, IndexList, Item, MultiIndex};
use osmo_bindings::OsmosisMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20::BalanceResponse as Cw20BalanceResponse;
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

use crate::hub::{PendingBatch, State};

const REPLY_SAVE_OSMOSIS_DENOM: u64 = 14508;
const REPLY_SAVE_CW20_ADDRESS: u64 = 14509;

/// Unwrap a `Reply` object to extract the response
/// TODO: Copied from larrys steakhouse. Move to protocol
pub(crate) fn unwrap_reply(reply: Reply) -> StdResult<SubMsgResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
}

pub fn save_cw20_address(
    deps: DepsMut,
    res: SubMsgResponse,
    item_key: &str,
) -> StdResult<Response> {
    let item: Item<Token> = Item::new(item_key);

    let address = parse_contract_addr_from_instantiate_event(deps.as_ref(), res.events)
        .map_err(|e| StdError::generic_err(format!("{}", e)))?;

    item.save(deps.storage, &Token::Cw20 { address })?;

    Ok(Response::default())
}

fn parse_osmosis_denom_from_event(response: SubMsgResponse) -> StdResult<String> {
    let event = response
        .events
        .iter()
        .find(|event| event.ty == "instantiate")
        .ok_or_else(|| StdError::generic_err("cannot find `instantiate` event"))?;

    let denom = &event
        .attributes
        .iter()
        .find(|attr| attr.key == "new_token_denom")
        .ok_or_else(|| StdError::generic_err("cannot find `_contract_address` attribute"))?
        .value;

    Ok(denom.to_string())
}

pub fn save_osmosis_denom(
    deps: DepsMut,
    env: Env,
    res: SubMsgResponse,
    item_key: &str,
) -> StdResult<Response> {
    let item: Item<Token> = Item::new(item_key);

    let denom = parse_osmosis_denom_from_event(res)?;

    item.save(deps.storage, &Token::Osmosis { denom })?;

    Ok(Response::default())
}

pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> StdResult<Response> {
    let res = unwrap_reply(reply.clone())?;
    match reply.id {
        REPLY_SAVE_OSMOSIS_DENOM => save_osmosis_denom(deps, env, res, &"token"),
        REPLY_SAVE_CW20_ADDRESS => save_cw20_address(deps, res, &"token"),
        id => Err(StdError::generic_err(format!(
            "invalid reply id: {}; must be 14508-14509",
            id
        ))),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum TokenInitInfo {
    Osmosis {
        subdenom: String,
    },
    Cw20 {
        label: String,
        admin: Option<String>,
        code_id: u64,
        cw20_init_msg: Cw20InstantiateMsg,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInstantiator {
    item_key: String,
    init_info: TokenInitInfo,
}

const TOKEN_ITEM_KEY: Item<String> = Item::new("token_item_key");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OsmosisCreateDenomMsg {
    sender: String,
    subdenom: String,
}

impl TokenInstantiator {
    pub fn instantiate(&self, deps: DepsMut, env: Env) -> StdResult<SubMsg> {
        TOKEN_ITEM_KEY.save(deps.storage, &self.item_key)?;

        match self.init_info.clone() {
            TokenInitInfo::Osmosis { subdenom } => Ok(SubMsg::reply_always(
                CosmosMsg::Stargate {
                    type_url: "/osmosis.tokenfactory.v1beta1.MsgCreateDenom".to_string(),
                    value: to_binary(&OsmosisCreateDenomMsg {
                        sender: env.contract.address.to_string(),
                        subdenom: subdenom.to_string(),
                    })?,
                },
                REPLY_SAVE_OSMOSIS_DENOM,
            )),
            TokenInitInfo::Cw20 {
                cw20_init_msg,
                label,
                admin,
                code_id,
            } => Ok(SubMsg::reply_always(
                WasmMsg::Instantiate {
                    admin: admin,
                    code_id: code_id,
                    msg: to_binary(&cw20_init_msg)?,
                    funds: vec![],
                    label: label.to_string(),
                },
                REPLY_SAVE_CW20_ADDRESS,
            )),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Token {
    Osmosis { denom: String },
    Cw20 { address: Addr },
}

impl ToString for Token {
    fn to_string(&self) -> String {
        match self {
            Token::Osmosis { denom } => denom.to_owned(),
            Token::Cw20 { address } => address.to_string(),
        }
    }
}

/// Instantiate osmosis token. Saves the Token object to the storage in the supplied item.
///
/// ## Arguments
/// * `deps` - Dependencies object
/// * `env` - Environment object
/// * `subdenom` - Sub-denomination of the token
/// * `item` - Item to save the `Token` object to
///
/// Returns OsmosisMsg to create the denom wrapped in a [`StdResult`].
pub fn init_osmosis_token(
    deps: DepsMut,
    env: Env,
    subdenom: String,
    item: Item<Token>,
) -> StdResult<OsmosisMsg> {
    item.save(
        deps.storage,
        &Token::Osmosis {
            denom: format!("factory/{}/{}", env.contract.address, subdenom),
        },
    )?;

    Ok(OsmosisMsg::CreateDenom { subdenom })
}

pub fn init_cw20_token(
    code_id: u64,
    label: String,
    cw20_init_msg: Cw20InstantiateMsg,
    admin: Option<String>,
    reply_id: u64,
) -> StdResult<SubMsg> {
    Ok(SubMsg::reply_always(
        WasmMsg::Instantiate {
            admin,
            code_id,
            msg: to_binary(&cw20_init_msg)?,
            funds: vec![],
            label,
        },
        reply_id,
    ))
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct OsmosisMintMsg {
    amount: Coin,
    sender: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct OsmosisBurnMsg {
    amount: Coin,
    sender: String,
}

pub(crate) struct PreviousBatchesIndexes<'a> {
    // pk goes to second tuple element
    pub reconciled: MultiIndex<'a, BooleanKey, Batch, Vec<u8>>,
}

impl<'a> IndexList<Batch> for PreviousBatchesIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Batch>> + '_> {
        let v: Vec<&dyn Index<Batch>> = vec![&self.reconciled];
        Box::new(v.into_iter())
    }
}

pub(crate) struct UnbondRequestsIndexes<'a> {
    // pk goes to second tuple element
    pub user: MultiIndex<'a, String, UnbondRequest, Vec<u8>>,
}

impl<'a> IndexList<UnbondRequest> for UnbondRequestsIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnbondRequest>> + '_> {
        let v: Vec<&dyn Index<UnbondRequest>> = vec![&self.user];
        Box::new(v.into_iter())
    }
}

impl Token {
    pub fn mint(&self, env: Env, amount: Uint128, recipient: String) -> StdResult<CosmosMsg> {
        match self {
            Token::Osmosis { denom } => Ok(CosmosMsg::Stargate {
                type_url: "/osmosis.tokenfactory.v1beta1.MsgMint".to_string(),
                value: to_binary(&OsmosisMintMsg {
                    amount: Coin {
                        denom: denom.to_string(),
                        amount,
                    },
                    sender: env.contract.address.to_string(),
                })?,
            }),
            Token::Cw20 { address } => Ok(WasmMsg::Execute {
                contract_addr: address.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint { amount, recipient })?,
                funds: vec![],
            }
            .into()),
        }
    }

    pub fn burn(&self, env: Env, amount: Uint128) -> StdResult<CosmosMsg> {
        match self {
            Token::Osmosis { denom } => Ok(CosmosMsg::Stargate {
                type_url: "/osmosis.tokenfactory.v1beta1.Msg/Burn".to_string(),
                value: to_binary(&OsmosisBurnMsg {
                    amount: Coin {
                        denom: denom.to_string(),
                        amount,
                    },
                    sender: env.contract.address.to_string(),
                })?,
            }),
            Token::Cw20 { address } => Ok(WasmMsg::Execute {
                contract_addr: address.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
                funds: vec![],
            }
            .into()),
        }
    }

    pub fn transfer(&self, env: Env, amount: Uint128, recipient: String) -> StdResult<CosmosMsg> {
        match self {
            Token::Osmosis { denom } => Ok(BankMsg::Send {
                to_address: recipient,
                amount: vec![Coin {
                    amount,
                    denom: denom.to_string(),
                }],
            }
            .into()),
            Token::Cw20 { address } => Ok(WasmMsg::Execute {
                contract_addr: address.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer { amount, recipient })?,
                funds: vec![],
            }
            .into()),
        }
    }

    pub fn query_balance(&self, querier: &QuerierWrapper, address: Addr) -> StdResult<Uint128> {
        match self {
            Token::Osmosis { denom } => {
                let query = BankQuery::Balance {
                    address: address.to_string(),
                    denom: denom.to_string(),
                };
                let res: BalanceResponse = querier.query(&query.into())?;
                Ok(res.amount.amount)
            }
            Token::Cw20 { address } => {
                let query = WasmQuery::Smart {
                    contract_addr: address.to_string(),
                    msg: to_binary(&Cw20QueryMsg::Balance {
                        address: address.to_string(),
                    })?,
                };
                let res: Cw20BalanceResponse = querier.query(&query.into())?;
                Ok(res.balance)
            }
        }
    }
}
