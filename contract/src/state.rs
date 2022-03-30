use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub(crate) struct State<'a> {
    pub steak_token: Item<'a, Addr>,
    pub workers: Item<'a, Vec<Addr>>,
    pub validators: Item<'a, Vec<String>>,
}

impl Default for State<'static> {
    fn default() -> Self {
        Self {
            steak_token: Item::new("steak_token"),
            workers: Item::new("workers"),
            validators: Item::new("validators"),
        }
    }
}
