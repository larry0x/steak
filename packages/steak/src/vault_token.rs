use std::vec;

use cosmwasm_std::{to_binary, Uint128, WasmMsg};
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

    pub fn burn_from(&self, amount: Uint128, burn_from_address: String) -> BurnFromTokenMsg {
        BurnFromTokenMsg {
            amount,
            token: self.to_owned(),
            burn_from_address,
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

pub struct BurnFromTokenMsg {
    pub burn_from_address: String,
    pub token: Token,
    pub amount: Uint128,
}

impl From<BurnFromTokenMsg> for OsmosisMsg {
    fn from(msg: BurnFromTokenMsg) -> OsmosisMsg {
        OsmosisMsg::BurnTokens {
            denom: msg.token.address,
            amount: msg.amount,
            burn_from_address: msg.burn_from_address,
        }
    }
}

impl From<BurnFromTokenMsg> for WasmMsg {
    fn from(msg: BurnFromTokenMsg) -> WasmMsg {
        WasmMsg::Execute {
            contract_addr: msg.token.address,
            msg: to_binary(&Cw20ExecuteMsg::BurnFrom {
                amount: msg.amount,
                owner: msg.burn_from_address,
            })
            .unwrap(),
            funds: vec![],
        }
    }
}
