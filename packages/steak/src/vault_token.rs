use std::{any::Any, vec};

use apollo_protocol::utils::parse_contract_addr_from_instantiate_event;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, CosmosMsg, DepsMut, Env, Reply, Response, StdError, StdResult,
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

impl Token {
    /// Instantiate osmosis token. Saves the Token object to the storage in the supplied item.
    ///
    /// ## Arguments
    /// * `deps` - Dependencies object
    /// * `env` - Environment object
    /// * `subdenom` - Sub-denomination of the token
    /// * `item` - Item to save the token to
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
        symbol: String,
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

    pub fn mint(&self, amount: Uint128, recipient: String) -> StdResult<MintMsg> {
        match self {
            Token::OsmosisToken { denom } => Ok(MintMsg::Osmosis(OsmosisMsg::MintTokens {
                denom: denom.clone(),
                amount,
                mint_to_address: recipient,
            })),
            Token::Cw20Token { address } => Ok(MintMsg::Cw20(WasmMsg::Execute {
                contract_addr: address.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint { amount, recipient })?,
                funds: vec![],
            })),
        }
    }

    pub fn burn_from(&self, amount: Uint128, burn_from_address: String) -> BurnFromTokenMsg {
        BurnFromTokenMsg {
            amount,
            token: self.to_owned(),
            burn_from_address,
        }
    }
}

pub enum MintMsg {
    Osmosis(OsmosisMsg),
    Cw20(WasmMsg),
}

impl<S> From<MintMsg> for CosmosMsg<S> {
    fn from(msg: MintMsg) -> Self {
        match msg {
            MintMsg::Osmosis(msg) => msg.into(),
            MintMsg::Cw20(msg) => msg.into(),
        }
    }
}

pub struct BurnFromTokenMsg {
    pub burn_from_address: String,
    pub token: Token,
    pub amount: Uint128,
}

impl From<BurnFromTokenMsg> for OsmosisMsg {
    fn from(msg: BurnFromTokenMsg) -> OsmosisMsg {
        OsmosisMsg::BurnTokens {
            denom: msg.token.to_string(),
            amount: msg.amount,
            burn_from_address: msg.burn_from_address,
        }
    }
}

impl From<BurnFromTokenMsg> for WasmMsg {
    fn from(msg: BurnFromTokenMsg) -> WasmMsg {
        WasmMsg::Execute {
            contract_addr: msg.token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::BurnFrom {
                amount: msg.amount,
                owner: msg.burn_from_address,
            })
            .unwrap(),
            funds: vec![],
        }
    }
}

pub struct TransferTokenMsg {
    pub token: Token,
    pub amount: Uint128,
    pub recipient: String,
    pub sender: String,
}

// impl From<TransferTokenMsg> for BankMsg {
//     fn from(msg: TransferTokenMsg) -> BankMsg {}
// }
