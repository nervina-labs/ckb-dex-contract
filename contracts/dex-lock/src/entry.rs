use crate::{error::Error, helper::DexArgs};
use ckb_std::{ckb_constants::Source, high_level::load_witness_args};

pub fn main() -> Result<(), Error> {
    let args = DexArgs::from_script()?;
    let witness_args = load_witness_args(0, Source::GroupInput)?;

    Ok(())
}
