use std::vec;

use cosmwasm_std::{to_binary, CosmosMsg, Empty, Uint128, WasmMsg};
use cw20_base::msg::ExecuteMsg as Cw20ExecuteMsg;
use osmo_bindings::OsmosisMsg;

#[derive(Clone)]
pub struct Token {
    pub address: String,
}

impl Token {
    pub fn new(address: String) -> Self {
        Token { address }
    }

    pub fn mint(&self, amount: Uint128, recipient: String) -> MintTokenMsg {
        MintTokenMsg {
            amount,
            token: self.to_owned(),
            recipient: recipient,
        }
    }
}

pub struct MintTokenMsg {
    pub token: Token,
    pub recipient: String,
    pub amount: Uint128,
}

impl From<MintTokenMsg> for OsmosisMsg {
    fn from(msg: MintTokenMsg) -> OsmosisMsg {
        OsmosisMsg::MintTokens {
            denom: msg.token.address,
            amount: msg.amount,
            mint_to_address: msg.recipient,
        }
    }
}

impl From<MintTokenMsg> for WasmMsg {
    fn from(msg: MintTokenMsg) -> WasmMsg {
        WasmMsg::Execute {
            contract_addr: msg.token.address,
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: msg.recipient,
                amount: msg.amount,
            })
            .unwrap(),
            funds: vec![],
        }
    }
}
