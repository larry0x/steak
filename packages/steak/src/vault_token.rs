use std::{
    any::Any,
    convert::{TryFrom, TryInto},
    vec,
};

use apollo_protocol::utils::parse_contract_addr_from_instantiate_event;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, Reply, Response, StdError, StdResult,
    SubMsg, SubMsgResponse, Uint128, WasmMsg,
};
use cw20_base::msg::ExecuteMsg as Cw20ExecuteMsg;
use cw_storage_plus::Item;
use osmo_bindings::OsmosisMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

const REPLY_SAVE_ADDRESS: u64 = 14509;

/// Unwrap a `Reply` object to extract the response
/// TODO: Move to protocol
pub(crate) fn unwrap_reply(reply: Reply) -> StdResult<SubMsgResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
}

pub fn save_vault_token_reply(
    deps: DepsMut,
    reply: Reply,
    item: Item<Token>,
) -> StdResult<Response> {
    if reply.id == REPLY_SAVE_ADDRESS {
        let address =
            parse_contract_addr_from_instantiate_event(deps.as_ref(), unwrap_reply(reply)?.events)
                .map_err(|e| StdError::generic_err(format!("{}", e)))?;
        item.save(deps.storage, &Token::Cw20Token { address })?;
    }
    Ok(Response::default())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Token {
    OsmosisToken { denom: String },
    Cw20Token { address: Addr },
}

impl ToString for Token {
    fn to_string(&self) -> String {
        match self {
            Token::OsmosisToken { denom } => denom.to_owned(),
            Token::Cw20Token { address } => address.to_string(),
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
        &Token::OsmosisToken {
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

impl Token {
    pub fn mint(&self, env: Env, amount: Uint128, recipient: String) -> StdResult<CosmosMsg> {
        match self {
            Token::OsmosisToken { denom } => Ok(CosmosMsg::Stargate {
                type_url: "/osmosis.tokenfactory.v1beta1.MsgMint".to_string(),
                value: to_binary(&OsmosisMintMsg {
                    amount: Coin {
                        denom: denom.to_string(),
                        amount,
                    },
                    sender: env.contract.address.to_string(),
                })?,
            }),
            Token::Cw20Token { address } => Ok(WasmMsg::Execute {
                contract_addr: address.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint { amount, recipient })?,
                funds: vec![],
            }
            .into()),
        }
    }

    pub fn burn(&self, env: Env, amount: Uint128) -> StdResult<CosmosMsg> {
        match self {
            Token::OsmosisToken { denom } => Ok(CosmosMsg::Stargate {
                type_url: "/osmosis.tokenfactory.v1beta1.Msg/Burn".to_string(),
                value: to_binary(&OsmosisBurnMsg {
                    amount: Coin {
                        denom: denom.to_string(),
                        amount,
                    },
                    sender: env.contract.address.to_string(),
                })?,
            }),
            Token::Cw20Token { address } => Ok(WasmMsg::Execute {
                contract_addr: address.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
                funds: vec![],
            }
            .into()),
        }
    }

    pub fn transfer(&self, env: Env, amount: Uint128, recipient: String) -> StdResult<CosmosMsg> {
        match self {
            Token::OsmosisToken { denom } => Ok(BankMsg::Send {
                to_address: recipient,
                amount: vec![Coin {
                    amount,
                    denom: denom.to_string(),
                }],
            }
            .into()),
            Token::Cw20Token { address } => Ok(WasmMsg::Execute {
                contract_addr: address.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer { amount, recipient })?,
                funds: vec![],
            }
            .into()),
        }
    }
}
