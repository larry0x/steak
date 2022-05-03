use std::marker::PhantomData;

use cw_storage_plus::{Prefixer, PrimaryKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BooleanKey {
    pub wrapped: Vec<u8>,
    pub data: PhantomData<bool>,
}

impl BooleanKey {
    pub fn new(val: bool) -> Self {
        BooleanKey {
            wrapped: if val {
                vec![1]
            } else {
                vec![0]
            },
            data: PhantomData,
        }
    }
}

impl From<bool> for BooleanKey {
    fn from(val: bool) -> Self {
        Self::new(val)
    }
}

impl<'a> PrimaryKey<'a> for BooleanKey {
    type Prefix = ();
    type SubPrefix = ();

    fn key(&self) -> Vec<&[u8]> {
        self.wrapped.key()
    }
}

impl<'a> Prefixer<'a> for BooleanKey {
    fn prefix(&self) -> Vec<&[u8]> {
        self.wrapped.prefix()
    }
}
