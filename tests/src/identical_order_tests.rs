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
const DEX_OWNER_LOCK_NOT_MATCH: i8 = 6;

fn create_identical_orders_context(pay_second_order: bool) -> (Context, TransactionView) {
    let mut context = Context::default();
    let dex_bin: Bytes = Loader::default().load_binary("dex-lock");
    let dex_out_point = context.deploy_cell(dex_bin);
    let dex_dep = CellDep::new_builder()
        .out_point(dex_out_point.clone())
        .build();

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
        .expect("owner lock");
    let buyer_lock = context
        .build_script(
            &always_success_out_point,
            rng.gen::<[u8; 20]>().to_vec().into(),
        )
        .expect("buyer lock");

    let order_args = DexArgs {
        owner_lock:     owner_lock.clone(),
        setup:          4,
        total_value:    1_0000_0000u128,
        receiver_lock:  None,
        unit_type_hash: None,
    };
    let dex_lock = context
        .build_script(&dex_out_point, order_args.to_vec().unwrap().into())
        .expect("dex lock");

    let asset_type = ScriptBuilder::default()
        .code_hash(Byte32::from_slice(&blake2b_256(ALWAYS_SUCCESS.clone())).unwrap())
        .hash_type(Byte::from(2u8))
        .build();

    let asset_data = Bytes::from(1u128.to_le_bytes().to_vec());
    let order1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(100_0000_0000u64.pack())
            .lock(dex_lock.clone())
            .type_(Some(asset_type.clone()).pack())
            .build(),
        asset_data.clone(),
    );
    let order2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(100_0000_0000u64.pack())
            .lock(dex_lock)
            .type_(Some(asset_type.clone()).pack())
            .build(),
        asset_data.clone(),
    );

    let inputs = vec![
        CellInput::new_builder().previous_output(order1).build(),
        CellInput::new_builder().previous_output(order2).build(),
    ];

    let seller_payment = |capacity: u64| {
        CellOutput::new_builder()
            .capacity(capacity.pack())
            .lock(owner_lock.clone())
            .build()
    };
    let buyer_asset = || {
        CellOutput::new_builder()
            .capacity(20_0000_0000u64.pack())
            .lock(buyer_lock.clone())
            .type_(Some(asset_type.clone()).pack())
            .build()
    };
    let buyer_change = |capacity: u64| {
        CellOutput::new_builder()
            .capacity(capacity.pack())
            .lock(buyer_lock.clone())
            .build()
    };

    let (outputs, outputs_data) = if pay_second_order {
        (
            vec![
                seller_payment(1_0000_0000),
                seller_payment(1_0000_0000),
                buyer_asset(),
                buyer_asset(),
                buyer_change(157_0000_0000),
            ],
            vec![
                Bytes::new(),
                Bytes::new(),
                asset_data.clone(),
                asset_data,
                Bytes::new(),
            ],
        )
    } else {
        (
            vec![
                seller_payment(1_0000_0000),
                buyer_asset(),
                buyer_asset(),
                buyer_change(158_0000_0000),
            ],
            vec![Bytes::new(), asset_data.clone(), asset_data, Bytes::new()],
        )
    };

    let witnesses = vec![Bytes::new(); inputs.len()];
    let tx = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_deps(vec![always_success_dep, dex_dep])
        .witnesses(witnesses.pack())
        .build();
    let tx = context.complete_tx(tx);

    (context, tx)
}

#[test]
fn test_identical_dex_locks_require_one_payment_each() {
    let (context, tx) = create_identical_orders_context(false);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, DEX_OWNER_LOCK_NOT_MATCH);
}

#[test]
fn test_identical_dex_locks_succeed_with_one_payment_each() {
    let (context, tx) = create_identical_orders_context(true);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("each order payment should pass verification");
}
