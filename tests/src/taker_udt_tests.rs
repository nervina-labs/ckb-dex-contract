use self::helper::DexArgs;

use super::*;
use ckb_testtool::builtin::ALWAYS_SUCCESS;
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};
use ckb_testtool::context::Context;
use rand::{thread_rng, Rng};

const MAX_CYCLES: u64 = 70_000_000;

#[repr(i8)]
#[derive(PartialEq, Eq, Clone, Copy)]
enum DexError {
    NoError,
    UnitTypeNotMatch = 11,
    TotalValueNotMatch,
}

fn create_test_context(error: DexError) -> (Context, TransactionView) {
    // deploy contract
    let mut context = Context::default();
    let dex_bin: Bytes = Loader::default().load_binary("dex-lock");
    let dex_out_point = context.deploy_cell(dex_bin);
    let dex_dep = CellDep::new_builder()
        .out_point(dex_out_point.clone())
        .build();

    let sudt_bin: Bytes = Loader::default().load_binary("sudt");
    let sudt_out_point = context.deploy_cell(sudt_bin);
    let sudt_dep = CellDep::new_builder()
        .out_point(dex_out_point.clone())
        .build();
    let sudt_type_script = context
        .build_script(&sudt_out_point, Bytes::from(vec![42]))
        .expect("script");

    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();
    let always_success_lock = context
        .build_script(&always_success_out_point, Default::default())
        .expect("script");

    let mut rng = thread_rng();
    let owner_lock = context
        .build_script(
            &always_success_out_point,
            rng.gen::<[u8; 20]>().to_vec().into(),
        )
        .expect("script");
    let buyer_lock = context
        .build_script(
            &always_success_out_point,
            rng.gen::<[u8; 20]>().to_vec().into(),
        )
        .expect("script");

    let input_token: u128 = 4000_0000_0000;
    let total_value: u128 = 1234_5678_0000;
    let change: u128 = input_token - total_value;

    let mut temp = [0u8; 32];
    temp.copy_from_slice(&sudt_type_script.calc_script_hash().as_bytes()[0..32]);

    let setup = 0b0000_0010;
    let dex_args = DexArgs {
        owner_lock: owner_lock.clone(),
        setup,
        total_value,
        receiver_lock: None,
        unit_type_hash: Some(temp),
    };
    let dex_lock_script = context
        .build_script(&dex_out_point, dex_args.to_vec().unwrap().into())
        .expect("script");

    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(dex_lock_script)
            .build(),
        Bytes::default(),
    );

    let input_out_point1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(4000u64.pack())
            .lock(always_success_lock)
            .type_(Some(sudt_type_script.clone()).pack())
            .build(),
        input_token.to_le_bytes().to_vec().into(),
    );

    let inputs = vec![
        CellInput::new_builder()
            .previous_output(input_out_point)
            .build(),
        CellInput::new_builder()
            .previous_output(input_out_point1)
            .build(),
    ];

    let first_output = if error == DexError::UnitTypeNotMatch {
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(owner_lock.clone())
            .build()
    } else {
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(owner_lock.clone())
            .type_(Some(sudt_type_script.clone()).pack())
            .build()
    };

    let outputs = vec![
        first_output,
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(buyer_lock.clone())
            .build(),
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .type_(Some(sudt_type_script.clone()).pack())
            .lock(buyer_lock)
            .build(),
    ];

    let first_output_data = if error == DexError::TotalValueNotMatch {
        Bytes::try_from((total_value - 1).to_le_bytes().to_vec()).unwrap()
    } else {
        Bytes::try_from(total_value.to_le_bytes().to_vec()).unwrap()
    };

    let outputs_data = vec![
        first_output_data,
        Bytes::default(),
        Bytes::try_from(change.to_le_bytes().to_vec()).unwrap(),
    ];

    let mut witnesses = vec![];
    for _ in 0..inputs.len() {
        witnesses.push(Bytes::new())
    }

    let cell_deps = vec![always_success_dep, dex_dep, sudt_dep];
    // build transaction
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_deps(cell_deps)
        .witnesses(witnesses.pack())
        .build();
    let tx = context.complete_tx(tx);

    // sign
    (context, tx)
}

#[test]
fn test_dex_taker_order_success() {
    let (context, tx) = create_test_context(DexError::NoError);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_dex_taker_order_unit_type_not_match_error() {
    let (context, tx) = create_test_context(DexError::UnitTypeNotMatch);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, DexError::UnitTypeNotMatch as i8);
}

#[test]
fn test_dex_taker_order_total_value_not_match_error() {
    let (context, tx) = create_test_context(DexError::TotalValueNotMatch);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, DexError::TotalValueNotMatch as i8);
}
