# Simple DEX Lock Contract 

> **Note: This code has not been audited by security agencies; please use it with caution.**

## Background

A simple token DEX contract designed to enable the sale of assets managed by type scripts such as NFTs, FTs, and SFTs.

## General Overview

- The so-called "DEX Contract" refers to a DEX lock, which we call **Simple DEX Lock (SDL)**.
- The trading logic is as follows: any type asset decoupled from a lock can be locked into the SDL. After the buyer pays the required cost, they can retrieve the asset within the type.
- The core definitions of SDL are:
  - Who can cancel an order.
  - The pricing unit and total price.
  - The recipient of the payment.
  - SDL does not concern itself with whether the asset for sale is an NFT, FT, or SFT, nor does it care about the unit price—only the total price.

## SDL Data Structure

```yaml
codehash:
    0x00000123456 # Deployment hash
args:
    owner_lock,
    setup_byte,
    total_value,
    receiver_lock, # Optional
    unit_type_hash   # Optional
```

### Definitions

`setup_byte`

- Size: 1 byte
- Meaning: Configuration description

| Bits | Meaning | MVP |
| --- | --- | --- |
| 0 | 0: `receiver_lock` does not exist; the receiver is the same as the `owner_lock`. 1: `receiver_lock` exists. | Only implement 0. |
| 1 | 0: Settlement in CKB without `unit_type`. 1: `unit_type_hash` exists, and compatible assets are settled using SUDT/UDT. | Only implement 0. |
| 2 | 0: Fungible token. 1: Non-fungible token. |  |
| 3 | Reserved. |  |
| 4-7 | Version (`uint4`). | 0000 |

`total_value`

- Size: `u128` (big-endian).

`owner_lock`

- Specifies the individual authorized to cancel the order.

`receiver_lock`

- Specifies the beneficiary. Optional; if absent, the beneficiary is the `owner_lock`.

`unit_type_hash`

- Size: 20 bytes.
- Specifies the pricing unit. Optional; if absent, the default is CKB. If present, verification is required to ensure it equals `typescript_hash[:20]`, and the default cell data must conform to the SUDT definition.

## Transaction Templates

### Listing (CKB)

```yaml
Input:
    xudt_cell: # Can be any type cell
        data: amount
        type: xudt_a
        lock: <any_lock>
    ckb_cell:
        ..
Output:
    xudt_cell:
        data: amount
        type: xudt_a
        lock: SDL
            owner_lock: <owner_lock>
            setup_byte: 0x00
            total_value: u128
    ckb_cell:
        ..
```

### Matching (CKB)

```yaml
Input:
    xudt_cell:
        capacity:
        data: amount
        type: xudt_a
        lock: SDL
            owner_lock: <owner_lock>
            setup_byte: 0x00
            total_value: u128
    ckb_cell:
        lock: <buyer_lock>
Output:
    ckb_cell:
        capacity: ~~<total_value>~~
        lock: <owner_lock>
    xudt_cell:
        data: amount
        type: xudt_a
        lock: <any_lock>
    ckb_cell: # Change
        lock: <any_lock>
```

**Contract Constraints**

- **Each SDL-managed input order and matching output must correspond one-to-one.**
  - This enables a single transaction to consume multiple orders.
  - Fungible token:
    - `input[k].SDL.total_value + input[k].capacity <= output[k].capacity`
  - Non-fungible token:
    - `input[k].SDL.total_value <= output[k].capacity`
  - `input[k].SDL.receiver_lock == output[k].lock`

### Canceling an Order

```yaml
Input:
    ckb_cell:
        lock: <owner_lock>
    xudt_cell:
        data: amount
        type: xudt_a
        lock: SDL
            owner_lock: <owner_lock>
            setup_byte: 0x00
            total_value: u128
Output:
    xudt_cell:
        data: amount
        type: xudt_a
        lock: <any_lock>
    ckb_cell:
        lock: <any_lock>
```

**Contract Constraints**

- At least one cell in the input must have an address equal to `owner_lock`.

---

## Features Not Yet Implemented

### Listing (Payment in XUDT with a Separate Recipient Address)

```yaml
Input:
    xudt_cell: # Can be any type cell
        data: amount
        type: xudt_a
        lock: <any_lock>

    ckb_cell:
        lock: <any_lock>
Output:
    xudt_cell:
        data: amount
        type: xudt_a
        lock: SDL
            owner_lock: <owner_lock>
            setup_byte: 0x03
            total_value: u128
            receiver_lock: <receiver_lock>
            unit_type_hash: <unit_type_hash>

    ckb_cell:
        lock: <any_lock>
```

### Matching (Payment in XUDT with a Separate Recipient Address)

```yaml
Input:
    xudt_cell:
        data: amount
        type: xudt_a
        lock: SDL
            owner_lock: <owner_lock>
            setup_byte: 0x03
            total_value: u128
            receiver_lock: <receiver_lock>
            unit_type_hash: <unit_type_hash>

    xudt_cell:
        data: amount_buyer
        type: xudt_b
        lock: <buyer_lock>

    ckb_cell:
        lock: <buyer_lock>
Output:
    xudt_cell:
        data: <total_value>
        type: **xudt_b (of_hash == unit_type_hash)**
        lock: <receiver_lock>

    ckb_cell: # Seller's CKB refund (tentative)
        capacity: size(SDL_cell) - size(plain_cell)
        lock: <receiver_lock>
    
    xudt_cell:
        data: amount
        type: xudt_a
        lock: <any_lock>

    xudt_cell: # Buyer's change
        data: amount_buyer - <total_value>
        type: xudt_b
        lock: <any_lock>

    ckb_cell: # Buyer's change
        lock: <any_lock>
```

**Contract Constraints**

- **Each SDL-managed input order and matching output must correspond one-to-one.**
  - This enables a single transaction to consume multiple orders.
  - `input[k].SDL.total_value <= output[k].data.amount`
  - `input[k].SDL.receiver_lock == output[k].lock`
  - `input[k].SDL.unit_type_hash == output[k].type_hash`




## How to use 
Build contracts:

``` sh
make build
```

Run tests:

``` sh
make test
```



