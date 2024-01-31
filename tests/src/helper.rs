#![allow(dead_code)]

use ckb_testtool::ckb_types::{packed::Script, prelude::Entity};

#[derive(Debug, Clone)]
pub struct DexArgs {
    pub owner_lock:     Script,
    pub setup:          u8,
    pub total_value:    u128,
    pub receiver_lock:  Option<Script>,
    pub unit_type_hash: Option<[u8; 20]>,
}

impl DexArgs {
    pub fn to_vec(&self) -> Vec<u8> {
        let mut ret: Vec<u8> = self.owner_lock.as_bytes().to_vec();
        ret.extend([self.setup]);
        ret.extend(self.total_value.to_be_bytes());
        if let Some(lock) = &self.receiver_lock {
            ret.extend(lock.as_bytes());
        }
        if let Some(unit_hash) = self.unit_type_hash {
            ret.extend(unit_hash);
        }
        ret
    }
}
