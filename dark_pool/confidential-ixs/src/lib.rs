use arcis::prelude::*;
use std::ops::BitAnd;

arcis_linker!();

const ORDER_BOOK_SIZE: usize = 16;

#[derive(ArcisObject, Copy, Clone)]
pub struct OrderBook {
    orders: [Order; ORDER_BOOK_SIZE],
}

#[derive(ArcisType, Copy, Clone)]
struct Order {
    size: mu64,
    bid: mbool,
    owner: mu128,
}

#[confidential]
fn add_order(order: Order, ob: &mut OrderBook) {
    let mut found: mbool = false.into();
    for i in 0..ORDER_BOOK_SIZE {
        let overwrite = ob.orders[i].size.eq(0).bitand(!found);
        ob.orders[i] = overwrite.select(order, ob.orders[i]);
        found = overwrite | found;
    }
}

#[confidential]
fn find_next_match(ob: &OrderBook) -> (u128, u128) {
    const EMPTY_OWNER: rust_types::u128 = rust_types::u128::MAX;

    let mut owner_match: (mu128, mu128) = (EMPTY_OWNER.into(), EMPTY_OWNER.into());
    for i in 0..ORDER_BOOK_SIZE {
        for j in 0..ORDER_BOOK_SIZE {
            arcis! {
                let non_zero = ob.orders[i].size > 0;
                let match_order = ob.orders[i].size == ob.orders[j].size
                    && ob.orders[i].bid
                    && !ob.orders[j].bid
                    && non_zero;
                owner_match = if match_order {
                    (ob.orders[i].owner, ob.orders[j].owner)
                } else {
                    owner_match
                }
            }
        }
    }

    (owner_match.0.reveal(), owner_match.1.reveal())
}
