//! Sealed-bid auction circuits: maintain highest and second-highest bids in an
//! MXE-encrypted `AuctionState`, then reveal only the winner and clearing price.
//! Bid amounts other than the revealed clearing price never leave MPC.
//! Walkthrough: ../../README.md.

use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    pub struct Bid {
        pub bidder: SerializedSolanaPublicKey,
        pub amount: u64,
    }

    pub struct AuctionState {
        pub highest_bid: u64,
        pub highest_bidder: SerializedSolanaPublicKey,
        pub second_highest_bid: u64,
        pub bid_count: u16,
    }

    pub struct AuctionResult {
        pub winner: SerializedSolanaPublicKey,
        pub payment_amount: u64,
    }

    /// Produces a zeroed `AuctionState` encrypted for the MXE. Reveals nothing.
    #[instruction]
    pub fn init_auction_state() -> Enc<Mxe, AuctionState> {
        let initial_state = AuctionState {
            highest_bid: 0,
            highest_bidder: SerializedSolanaPublicKey { lo: 0, hi: 0 },
            second_highest_bid: 0,
            bid_count: 0,
        };
        Mxe::get().from_arcis(initial_state)
    }

    /// Folds one bid into the highest/second-highest tracking. Reveals nothing;
    /// the updated state is re-encrypted for the MXE.
    #[instruction]
    pub fn place_bid(
        bid_ctxt: Enc<Shared, Bid>,
        state_ctxt: Enc<Mxe, AuctionState>,
    ) -> Enc<Mxe, AuctionState> {
        let bid = bid_ctxt.to_arcis();
        let mut state = state_ctxt.to_arcis();

        if bid.amount > state.highest_bid {
            state.second_highest_bid = state.highest_bid;
            state.highest_bid = bid.amount;
            state.highest_bidder = bid.bidder;
        } else if bid.amount > state.second_highest_bid {
            state.second_highest_bid = bid.amount;
        }

        state.bid_count += 1;

        state_ctxt.owner.from_arcis(state)
    }

    /// Reveals the winner and their own bid as the price (first-price rule).
    #[instruction]
    pub fn determine_winner_first_price(state_ctxt: Enc<Mxe, AuctionState>) -> AuctionResult {
        let state = state_ctxt.to_arcis();

        AuctionResult {
            winner: state.highest_bidder,
            payment_amount: state.highest_bid,
        }
        .reveal()
    }

    /// Reveals the winner and the second-highest bid as the price (Vickrey rule);
    /// the winning bid amount itself stays encrypted.
    #[instruction]
    pub fn determine_winner_vickrey(state_ctxt: Enc<Mxe, AuctionState>) -> AuctionResult {
        let state = state_ctxt.to_arcis();

        AuctionResult {
            winner: state.highest_bidder,
            payment_amount: state.second_highest_bid,
        }
        .reveal()
    }
}
