use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    /// Bidder pubkey split into two u128s (Arcis encrypts each primitive separately).
    pub struct Bid {
        pub bidder_lo: u128,
        pub bidder_hi: u128,
        pub amount: u64,
    }

    pub struct AuctionState {
        pub highest_bid: u64,
        pub highest_bidder_lo: u128,
        pub highest_bidder_hi: u128,
        pub second_highest_bid: u64,
        pub bid_count: u8,
    }

    pub struct AuctionResult {
        pub winner_lo: u128,
        pub winner_hi: u128,
        pub payment_amount: u64,
    }

    #[instruction]
    pub fn init_auction_state(mxe: Mxe) -> Enc<Mxe, AuctionState> {
        let initial_state = AuctionState {
            highest_bid: 0,
            highest_bidder_lo: 0,
            highest_bidder_hi: 0,
            second_highest_bid: 0,
            bid_count: 0,
        };
        mxe.from_arcis(initial_state)
    }

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
            state.highest_bidder_lo = bid.bidder_lo;
            state.highest_bidder_hi = bid.bidder_hi;
        } else if bid.amount > state.second_highest_bid {
            state.second_highest_bid = bid.amount;
        }

        state.bid_count += 1;

        state_ctxt.owner.from_arcis(state)
    }

    /// Winner pays their bid.
    #[instruction]
    pub fn determine_winner_first_price(state_ctxt: Enc<Mxe, AuctionState>) -> AuctionResult {
        let state = state_ctxt.to_arcis();

        AuctionResult {
            winner_lo: state.highest_bidder_lo,
            winner_hi: state.highest_bidder_hi,
            payment_amount: state.highest_bid,
        }
        .reveal()
    }

    /// Winner pays second-highest bid (incentivizes truthful bidding).
    #[instruction]
    pub fn determine_winner_vickrey(state_ctxt: Enc<Mxe, AuctionState>) -> AuctionResult {
        let state = state_ctxt.to_arcis();

        AuctionResult {
            winner_lo: state.highest_bidder_lo,
            winner_hi: state.highest_bidder_hi,
            payment_amount: state.second_highest_bid,
        }
        .reveal()
    }
}
