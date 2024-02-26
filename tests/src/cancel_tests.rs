use self::helper::DexArgs;

use super::*;
use ckb_testtool::builtin::ALWAYS_SUCCESS;
use ckb_testtool::ckb_hash::blake2b_256;
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};
use ckb_testtool::context::Context;
use rand::{thread_rng, Rng};

const MAX_CYCLES: u64 = 70_000_000;

// error numbers
const LOCK_ARGS_INVALID: i8 = 5;
const DEX_FT_TOTAL_VALUE_NOT_MATCH: i8 = 7;

#[derive(PartialEq, Eq, Clone, Copy)]
enum DexError {
    NoError,
    LockArgsInvalid,
    DexFTTotalValueNotMatch,
}

fn create_test_context(error: DexError) -> (Context, TransactionView) {
    // deploy contract
    let mut context = Context::default();
    let dex_bin: Bytes = Loader::default().load_binary("dex-lock");
    let dex_out_point = context.deploy_cell(dex_bin);
    let dex_dep = CellDep::new_builder()
        .out_point(dex_out_point.clone())
        .build();

    let always_success_code_hash = blake2b_256(ALWAYS_SUCCESS.clone());
    let always_success_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());
    let always_success_dep = CellDep::new_builder()
        .out_point(always_success_out_point.clone())
        .build();

    let mut rng = thread_rng();
    let owner_lock = context
        .build_script(
            &always_success_out_point,
            rng.gen::<[u8; 20]>().to_vec().into(),
        )
        .expect("script");

    let dex_args1 = DexArgs {
        owner_lock:     owner_lock.clone(),
        setup:          0u8,
        total_value:    1234_5678_0000u128,
        receiver_lock:  None,
        unit_type_hash: None,
    };
    let dex_lock_script1 = context
        .build_script(&dex_out_point, dex_args1.to_vec().into())
        .expect("script");

    let dex_args2 = DexArgs {
        owner_lock:     owner_lock.clone(),
        setup:          0u8,
        total_value:    9_8765_0000_1234u128,
        receiver_lock:  None,
        unit_type_hash: None,
    };
    let mut dex_args2_vec = dex_args2.to_vec();
    if error == DexError::LockArgsInvalid {
        dex_args2_vec.reverse();
    }
    let dex_lock_script2 = context
        .build_script(&dex_out_point, dex_args2_vec.into())
        .expect("script");

    let asset_type = ScriptBuilder::default()
        .code_hash(Byte32::from_slice(&always_success_code_hash).unwrap())
        .hash_type(Byte::from(2u8))
        .build();

    let asset_amount1 = Bytes::try_from(1000_0000_0000u128.to_le_bytes().to_vec()).unwrap();
    let input_out_point1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(300_0000_0000u64.pack())
            .lock(dex_lock_script1)
            .type_(Some(asset_type.clone()).pack())
            .build(),
        asset_amount1,
    );
    let asset_amount2 = Bytes::try_from(3456_0000_0000u128.to_le_bytes().to_vec()).unwrap();
    let input_out_point2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(240_0000_0000u64.pack())
            .lock(dex_lock_script2)
            .type_(Some(asset_type.clone()).pack())
            .build(),
        asset_amount2,
    );
    let input_out_point3 = context.create_cell(
        CellOutput::new_builder()
            .capacity(100_0000_0000u64.pack())
            .lock(owner_lock.clone())
            .build(),
        Bytes::new(),
    );

    let mut inputs = vec![
        CellInput::new_builder()
            .previous_output(input_out_point1)
            .build(),
        CellInput::new_builder()
            .previous_output(input_out_point2)
            .build(),
    ];
    if error != DexError::DexFTTotalValueNotMatch {
        inputs.push(
            CellInput::new_builder()
                .previous_output(input_out_point3)
                .build(),
        )
    }

    let output_asset_amount = Bytes::try_from(
        (1000_0000_0000u128 + 3456_0000_0000u128)
            .to_le_bytes()
            .to_vec(),
    )
    .unwrap();
    let outputs = vec![
        CellOutput::new_builder()
            .capacity(240_0000_0000u64.pack())
            .lock(owner_lock.clone())
            .type_(Some(asset_type).pack())
            .build(),
        CellOutput::new_builder()
            .capacity(400_0000_0000u64.pack())
            .lock(owner_lock)
            .build(),
    ];

    let outputs_data = vec![output_asset_amount, Bytes::new()];

    let mut witnesses = vec![];
    for _ in 0..inputs.len() {
        witnesses.push(Bytes::new())
    }

    let cell_deps = vec![always_success_dep, dex_dep];
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
fn test_dex_cancel_order_success() {
    let (context, tx) = create_test_context(DexError::NoError);
    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_dex_cancel_order_lock_args_error() {
    let (context, tx) = create_test_context(DexError::LockArgsInvalid);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, LOCK_ARGS_INVALID);
}

#[test]
fn test_dex_cancel_order_owner_lock_not_match_error() {
    let (context, tx) = create_test_context(DexError::DexFTTotalValueNotMatch);
    // run
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, DEX_FT_TOTAL_VALUE_NOT_MATCH);
}
