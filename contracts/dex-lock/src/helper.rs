use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, packed::Script, prelude::*},
    high_level::{load_cell_lock, load_script, QueryIter},
};

use crate::error::Error;

pub fn parse_array<const N: usize>(arr: &[u8]) -> Result<[u8; N], Error> {
    arr.try_into().map_err(|_| Error::Encoding)
}

const MIN_ARGS_SIZE: usize = 66;
#[derive(Debug, Clone)]
pub struct DexArgs {
    // the minimum length of serialized lock script is 49bytes
    pub owner_lock:     Script,
    pub setup:          u8,
    pub total_value:    u128,
    pub receiver_lock:  Option<Script>,
    pub unit_type_hash: Option<[u8; 20]>,
}

impl DexArgs {
    pub fn from_script() -> Result<Self, Error> {
        let data: Bytes = load_script()?.args().unpack();
        if data.len() < MIN_ARGS_SIZE {
            return Err(Error::LockArgsInvalid);
        }
        let owner_size = u32::from_le_bytes(parse_array::<4>(&data[0..4])?) as usize;
        let required_size = owner_size + 17;
        if data.len() < required_size {
            return Err(Error::LockArgsInvalid);
        }

        let owner_lock = Script::from_slice(&data[..owner_size]).map_err(|_e| Error::Encoding)?;
        let setup = data[owner_size];
        if setup != 0 {
            return Err(Error::DexSetupInvalid);
        }
        let total_value =
            u128::from_be_bytes(parse_array::<16>(&data[owner_size + 1..required_size])?);

        let option = &data[required_size..];
        let mut receiver_lock = None;
        let mut unit_type_hash = None;
        if option.len() == 20 {
            unit_type_hash = Some(parse_array::<20>(option)?);
        } else if option.len() > required_size + 4 {
            let receiver_size = u32::from_le_bytes(parse_array::<4>(&option[0..4])?) as usize;
            if option.len() < receiver_size {
                return Err(Error::LockArgsInvalid);
            }
            receiver_lock =
                Some(Script::from_slice(&option[..receiver_size]).map_err(|_e| Error::Encoding)?);

            if option.len() == receiver_size + 20 {
                unit_type_hash = Some(parse_array::<20>(&option[option.len() - 20..])?)
            }
        }

        Ok(DexArgs {
            owner_lock,
            setup,
            total_value,
            receiver_lock,
            unit_type_hash,
        })
    }
}

pub fn position_dex_lock_in_inputs() -> Result<usize, Error> {
    let current_lock = load_script()?;
    QueryIter::new(load_cell_lock, Source::Input)
        .position(|lock| lock.as_slice() == current_lock.as_slice())
        .ok_or(Error::IndexOutOfBound)
}

pub fn inputs_contain_owner_cell(args: &DexArgs) -> bool {
    QueryIter::new(load_cell_lock, Source::Input)
        .any(|lock| lock.as_slice() == args.owner_lock.as_slice())
}
