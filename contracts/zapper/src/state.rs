use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;

pub(crate) struct State<'a> {
    /// Address of the Steak Hub
    pub steak_hub: Item<'a, Addr>,
    /// Address of the Steak token
    pub steak_token: Item<'a, Addr>,
    /// Address of the Astroport Router contract
    pub astro_router: Item<'a, Addr>,
    /// User who will receive the minted Steak tokens
    pub receiver: Item<'a, Addr>,
    /// Minimum amount of Steak token to receive
    pub minimum_received: Item<'a, Uint128>,
}

impl Default for State<'static> {
    fn default() -> Self {
        Self {
            steak_hub: Item::new("steak_hub"),
            steak_token: Item::new("steak_token"),
            astro_router: Item::new("astroport_router"),
            receiver: Item::new("receiver"),
            minimum_received: Item::new("minimum_received"),
        }
    }
}
