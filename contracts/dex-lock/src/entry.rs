use crate::{
    error::Error,
    helper::{inputs_contain_owner_cell, position_dex_lock_in_inputs, DexArgs},
};
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::Entity,
    high_level::{load_cell_capacity, load_cell_data, load_cell_lock, load_cell_type_hash},
};

pub const UDT_AMOUNT_LEN: usize = 16;

pub fn main() -> Result<(), Error> {
    let args = DexArgs::from_script()?;
    // When the inputs contain a cell whose lock script is owner, it means that the owner can do
    // anything including cancel the order
    if inputs_contain_owner_cell(&args) {
        return Ok(());
    }

    // The buyer must pay the specified amount of assets(CKB, UDT, etc.) in the DEX lock script args
    // to the seller's lock script
    let dex_index = position_dex_lock_in_inputs()?;
    let output_lock = load_cell_lock(dex_index, Source::Output)?;
    if args.owner_lock.as_slice() != output_lock.as_slice() {
        return Err(Error::DexOwnerLockNotMatch);
    }

    if let Some(unit_type_hash) = args.unit_type_hash {
        if let Some(type_hash) = load_cell_type_hash(dex_index, Source::Output)? {
            if type_hash != unit_type_hash {
                return Err(Error::UnitTypeNotMatch);
            } else {
                let mut buf = [0u8; UDT_AMOUNT_LEN];
                let data = load_cell_data(dex_index, Source::Output).unwrap();
                // not check the data length, because it's checked by xudt script
                buf.copy_from_slice(&data);
                let amount = u128::from_le_bytes(buf);
                if amount < args.total_value {
                    return Err(Error::TotalValueNotMatch);
                }
            }
        } else {
            return Err(Error::UnitTypeNotMatch);
        }
    } else {
        let dex_input_capacity = load_cell_capacity(dex_index, Source::Input)? as u128;
        let output_capacity = load_cell_capacity(dex_index, Source::Output)? as u128;

        if args.is_nft() {
            if args.total_value > output_capacity {
                return Err(Error::DexNFTTotalValueNotMatch);
            }
        } else if args.is_udt() {
            // Prevent total_value(u128) from overflowing
            let total_capacity = args
                .total_value
                .checked_add(dex_input_capacity)
                .ok_or(Error::TotalValueOverflow)?;
            if total_capacity > output_capacity {
                return Err(Error::DexFTTotalValueNotMatch);
            }
        }
    }

    Ok(())
}
