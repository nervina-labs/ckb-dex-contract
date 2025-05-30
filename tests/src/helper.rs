#![allow(dead_code)]

use ckb_testtool::ckb_types::{packed::Script, prelude::Entity};

#[derive(Debug, Clone)]
pub struct DexArgs {
    pub owner_lock:     Script,
    pub setup:          u8,
    pub total_value:    u128,
    pub receiver_lock:  Option<[u8; 32]>,
    pub unit_type_hash: Option<[u8; 32]>,
}

#[derive(Clone, Copy, Debug)]
pub enum Error {
    LockArgsInvalid,
}

impl DexArgs {
    pub fn to_vec(&self) -> Result<Vec<u8>, Error> {
        let mut ret: Vec<u8> = self.owner_lock.as_bytes().to_vec();
        ret.extend([self.setup]);
        ret.extend(self.total_value.to_be_bytes());

        let receiver_lock_flag = (self.setup & 0b0000_0001) != 0;

        // not support receiver_lock_flag now
        if receiver_lock_flag {
            if let Some(lock) = &self.receiver_lock {
                ret.extend(lock);
            } else {
                return Err(Error::LockArgsInvalid);
            }
        }

        let unit_type_hash_flag: bool = (self.setup & 0b0000_0010) != 0;

        if unit_type_hash_flag {
            if let Some(unit_hash) = &self.unit_type_hash {
                ret.extend(unit_hash);
            } else {
                return Err(Error::LockArgsInvalid);
            }
        }

        Ok(ret)
    }
}
