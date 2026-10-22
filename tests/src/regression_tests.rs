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

// error numbers, mirrors contracts/dex-lock/src/error.rs
const DEX_OWNER_LOCK_NOT_MATCH: i8 = 6;
const DEX_FT_TOTAL_VALUE_NOT_MATCH: i8 = 7;

const CKB: u64 = 1_0000_0000;

struct Env {
    context:      Context,
    dex_dep:      CellDep,
    as_dep:       CellDep,
    owner_lock:   Script,
    buyer_lock:   Script,
    asset_type:   Script,
    unit_type:    Script,
    dex_outpoint: OutPoint,
}

impl Env {
    fn new() -> Self {
        let mut context = Context::default();
        let dex_outpoint = context.deploy_cell(Loader::default().load_binary("dex-lock"));
        let dex_dep = CellDep::new_builder()
            .out_point(dex_outpoint.clone())
            .build();
        let as_outpoint = context.deploy_cell(ALWAYS_SUCCESS.clone());
        let as_dep = CellDep::new_builder()
            .out_point(as_outpoint.clone())
            .build();

        let mut rng = thread_rng();
        let owner_lock = context
            .build_script(&as_outpoint, rng.gen::<[u8; 20]>().to_vec().into())
            .expect("owner lock");
        let buyer_lock = context
            .build_script(&as_outpoint, rng.gen::<[u8; 20]>().to_vec().into())
            .expect("buyer lock");
        // sold asset and payment asset must be distinguishable
        let asset_type = context
            .build_script(&as_outpoint, Bytes::from(vec![1u8]))
            .expect("asset type");
        let unit_type = context
            .build_script(&as_outpoint, Bytes::from(vec![2u8]))
            .expect("unit type");

        Env {
            context,
            dex_dep,
            as_dep,
            owner_lock,
            buyer_lock,
            asset_type,
            unit_type,
            dex_outpoint,
        }
    }

    fn dex_lock(&mut self, args: &DexArgs) -> Script {
        self.context
            .build_script(&self.dex_outpoint, args.to_vec().unwrap().into())
            .expect("dex lock")
    }

    fn owner_cell(&self, capacity: u64) -> CellOutput {
        CellOutput::new_builder()
            .capacity(capacity.pack())
            .lock(self.owner_lock.clone())
            .build()
    }

    fn buyer_cell(&self, capacity: u64) -> CellOutput {
        CellOutput::new_builder()
            .capacity(capacity.pack())
            .lock(self.buyer_lock.clone())
            .build()
    }

    fn buyer_asset_cell(&self, capacity: u64, data: Bytes) -> (CellOutput, Bytes) {
        (
            CellOutput::new_builder()
                .capacity(capacity.pack())
                .lock(self.buyer_lock.clone())
                .type_(Some(self.asset_type.clone()).pack())
                .build(),
            data,
        )
    }

    /// Builds the transaction. Dex order inputs always occupy input indices `0..orders.len()`,
    /// so `output[k]` is what the contract validates against `input[k]`.
    fn finish(
        &mut self,
        orders: Vec<(u64, Option<Script>, Bytes)>,
        dex_lock: Script,
        outputs: Vec<CellOutput>,
        outputs_data: Vec<Bytes>,
    ) -> TransactionView {
        let mut inputs: Vec<CellInput> = Vec::new();
        let mut dex_capacity: u64 = 0;

        for (capacity, type_, data) in orders {
            dex_capacity += capacity;
            let outpoint = self.context.create_cell(
                CellOutput::new_builder()
                    .capacity(capacity.pack())
                    .lock(dex_lock.clone())
                    .type_(type_.pack())
                    .build(),
                data,
            );
            inputs.push(CellInput::new_builder().previous_output(outpoint).build());
        }

        let outputs_capacity: u64 = outputs
            .iter()
            .map(|o| Unpack::<u64>::unpack(&o.capacity()))
            .sum();
        let funding = outputs_capacity.saturating_sub(dex_capacity);
        if funding > 0 {
            let outpoint = self
                .context
                .create_cell(self.buyer_cell(funding), Bytes::new());
            inputs.push(CellInput::new_builder().previous_output(outpoint).build());
        }

        let witnesses = vec![Bytes::new(); inputs.len()];
        let tx = TransactionBuilder::default()
            .inputs(inputs)
            .outputs(outputs)
            .outputs_data(outputs_data.pack())
            .cell_deps(vec![self.as_dep.clone(), self.dex_dep.clone()])
            .witnesses(witnesses.pack())
            .build();
        self.context.complete_tx(tx)
    }
}

fn amount_data(amount: u128) -> Bytes {
    Bytes::from(amount.to_le_bytes().to_vec())
}

// ---------------------------------------------------------------------------
// Orders that share a lock script but hold different capacities must each be
// settled against their own input capacity.
// ---------------------------------------------------------------------------

fn mixed_capacity_group(real_payment: u64) -> (Context, TransactionView) {
    let mut env = Env::new();
    let args = DexArgs {
        owner_lock:     env.owner_lock.clone(),
        setup:          0,
        total_value:    100 * CKB as u128,
        receiver_lock:  None,
        unit_type_hash: None,
    };
    let dex_lock = env.dex_lock(&args);

    // input[0] holds 63 CKB, input[1] holds 200 CKB, both under the same lock.
    let orders = vec![
        (63 * CKB, Some(env.asset_type.clone()), amount_data(1000)),
        (200 * CKB, Some(env.asset_type.clone()), amount_data(1000)),
    ];

    let (asset_out, asset_data) = env.buyer_asset_cell(100 * CKB, amount_data(1000));
    let outputs = vec![
        env.owner_cell(163 * CKB), // input[0]: 100 + 63
        env.owner_cell(real_payment),
        asset_out,
        env.buyer_cell(50 * CKB),
    ];
    let outputs_data = vec![Bytes::new(), Bytes::new(), asset_data, Bytes::new()];

    let tx = env.finish(orders, dex_lock, outputs, outputs_data);
    (env.context, tx)
}

#[test]
fn test_each_order_is_settled_with_its_own_capacity() {
    let (context, tx) = mixed_capacity_group(163 * CKB);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, DEX_FT_TOTAL_VALUE_NOT_MATCH);
}

#[test]
fn test_mixed_capacity_group_succeeds_with_full_payment() {
    let (context, tx) = mixed_capacity_group(300 * CKB);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("real order must be paid with its own input capacity");
}

// ---------------------------------------------------------------------------
// Multiple identical orders inside a single lock script group.
// ---------------------------------------------------------------------------

fn identical_nft_orders(orders_count: usize, payments: usize) -> (Context, TransactionView) {
    let mut env = Env::new();
    let args = DexArgs {
        owner_lock:     env.owner_lock.clone(),
        setup:          4,
        total_value:    1 * CKB as u128,
        receiver_lock:  None,
        unit_type_hash: None,
    };
    let dex_lock = env.dex_lock(&args);

    let orders = vec![(100 * CKB, Some(env.asset_type.clone()), amount_data(1)); orders_count];

    let mut outputs: Vec<CellOutput> = Vec::new();
    let mut outputs_data: Vec<Bytes> = Vec::new();
    for _ in 0..payments {
        outputs.push(env.owner_cell(1 * CKB));
        outputs_data.push(Bytes::new());
    }
    for _ in 0..orders_count {
        let (out, data) = env.buyer_asset_cell(99 * CKB, amount_data(1));
        outputs.push(out);
        outputs_data.push(data);
    }

    let tx = env.finish(orders, dex_lock, outputs, outputs_data);
    (env.context, tx)
}

#[test]
fn test_identical_orders_require_payment_each() {
    let (context, tx) = identical_nft_orders(3, 1);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, DEX_OWNER_LOCK_NOT_MATCH);
}

#[test]
fn test_identical_orders_succeed_with_payment_each() {
    let (context, tx) = identical_nft_orders(3, 3);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("each order needs its own payment");
}

// ---------------------------------------------------------------------------
// UDT denominated orders (setup bit 0b0000_0010).
// ---------------------------------------------------------------------------

fn udt_denominated_orders(
    orders_count: usize,
    payments: usize,
    payment_capacity: u64,
) -> (Context, TransactionView) {
    let mut env = Env::new();
    let mut unit_type_hash = [0u8; 32];
    unit_type_hash.copy_from_slice(&env.unit_type.calc_script_hash().as_bytes()[0..32]);

    let args = DexArgs {
        owner_lock:     env.owner_lock.clone(),
        setup:          0b0000_0010,
        total_value:    1000u128,
        receiver_lock:  None,
        unit_type_hash: Some(unit_type_hash),
    };
    let dex_lock = env.dex_lock(&args);

    let orders = vec![(197 * CKB, Some(env.asset_type.clone()), amount_data(1000)); orders_count];

    let mut outputs: Vec<CellOutput> = Vec::new();
    let mut outputs_data: Vec<Bytes> = Vec::new();
    for _ in 0..payments {
        outputs.push(
            CellOutput::new_builder()
                .capacity(payment_capacity.pack())
                .lock(env.owner_lock.clone())
                .type_(Some(env.unit_type.clone()).pack())
                .build(),
        );
        outputs_data.push(amount_data(1000));
    }
    for _ in 0..orders_count {
        let (out, data) = env.buyer_asset_cell(142 * CKB, amount_data(1000));
        outputs.push(out);
        outputs_data.push(data);
    }
    outputs.push(env.buyer_cell(50 * CKB));
    outputs_data.push(Bytes::new());

    let tx = env.finish(orders, dex_lock, outputs, outputs_data);
    (env.context, tx)
}

#[test]
fn test_udt_orders_require_payment_each() {
    let (context, tx) = udt_denominated_orders(2, 1, 197 * CKB);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, DEX_OWNER_LOCK_NOT_MATCH);
}

#[test]
fn test_udt_denominated_order_with_full_capacity_succeeds() {
    let (context, tx) = udt_denominated_orders(1, 1, 197 * CKB);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("seller must get the listing capacity back");
}

// ---------------------------------------------------------------------------
// The owner bypass must stay scoped to the lock group it belongs to.
// ---------------------------------------------------------------------------

#[test]
fn test_owner_lock_is_scoped_to_its_own_group() {
    let mut env = Env::new();
    let mut rng = thread_rng();
    let as_outpoint = env.context.deploy_cell(ALWAYS_SUCCESS.clone());
    let other_owner = env
        .context
        .build_script(&as_outpoint, rng.gen::<[u8; 20]>().to_vec().into())
        .expect("other owner");

    let args = DexArgs {
        owner_lock:     env.owner_lock.clone(),
        setup:          4,
        total_value:    1 * CKB as u128,
        receiver_lock:  None,
        unit_type_hash: None,
    };
    let dex_lock = env.dex_lock(&args);

    let orders = vec![(100 * CKB, Some(env.asset_type.clone()), amount_data(1))];

    // Only an unrelated lock is present among the inputs.
    let tx = {
        let mut inputs: Vec<CellInput> = Vec::new();
        for (capacity, type_, data) in orders {
            let outpoint = env.context.create_cell(
                CellOutput::new_builder()
                    .capacity(capacity.pack())
                    .lock(dex_lock.clone())
                    .type_(type_.pack())
                    .build(),
                data,
            );
            inputs.push(CellInput::new_builder().previous_output(outpoint).build());
        }
        let witness_outpoint = env.context.create_cell(
            CellOutput::new_builder()
                .capacity((100 * CKB).pack())
                .lock(other_owner.clone())
                .build(),
            Bytes::new(),
        );
        inputs.push(
            CellInput::new_builder()
                .previous_output(witness_outpoint)
                .build(),
        );

        let (asset_out, asset_data) = env.buyer_asset_cell(99 * CKB, amount_data(1));
        let outputs = vec![
            CellOutput::new_builder()
                .capacity((100 * CKB).pack())
                .lock(other_owner.clone())
                .build(),
            asset_out,
        ];
        let outputs_data = vec![Bytes::new(), asset_data];

        let witnesses = vec![Bytes::new(); inputs.len()];
        let tx = TransactionBuilder::default()
            .inputs(inputs)
            .outputs(outputs)
            .outputs_data(outputs_data.pack())
            .cell_deps(vec![env.as_dep.clone(), env.dex_dep.clone()])
            .witnesses(witnesses.pack())
            .build();
        env.context.complete_tx(tx)
    };

    let err = env.context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, DEX_OWNER_LOCK_NOT_MATCH);
}

// ---------------------------------------------------------------------------
// xUDT extension data: a payment cell carrying trailing data must still be
// parsed instead of panicking.
// ---------------------------------------------------------------------------

#[test]
fn test_udt_payment_with_extension_data_is_accepted() {
    let mut env = Env::new();
    let mut unit_type_hash = [0u8; 32];
    unit_type_hash.copy_from_slice(&env.unit_type.calc_script_hash().as_bytes()[0..32]);

    let args = DexArgs {
        owner_lock:     env.owner_lock.clone(),
        setup:          0b0000_0010,
        total_value:    1000u128,
        receiver_lock:  None,
        unit_type_hash: Some(unit_type_hash),
    };
    let dex_lock = env.dex_lock(&args);

    let orders = vec![(197 * CKB, Some(env.asset_type.clone()), amount_data(1000))];

    let mut ext = amount_data(1000).to_vec();
    ext.extend_from_slice(&[9u8; 32]);

    let outputs = vec![CellOutput::new_builder()
        .capacity((197 * CKB).pack())
        .lock(env.owner_lock.clone())
        .type_(Some(env.unit_type.clone()).pack())
        .build()];
    let outputs_data = vec![Bytes::from(ext)];

    let tx = env.finish(orders, dex_lock, outputs, outputs_data);
    env.context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("extension data must be tolerated");
}
