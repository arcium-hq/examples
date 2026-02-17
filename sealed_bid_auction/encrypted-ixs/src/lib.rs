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
        pub bid_count: u8,
    }

    pub struct AuctionResult {
        pub winner: SerializedSolanaPublicKey,
        pub payment_amount: u64,
    }

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

    /// Winner pays their bid.
    #[instruction]
    pub fn determine_winner_first_price(state_ctxt: Enc<Mxe, AuctionState>) -> AuctionResult {
        let state = state_ctxt.to_arcis();

        AuctionResult {
            winner: state.highest_bidder,
            payment_amount: state.highest_bid,
        }
        .reveal()
    }

    /// Winner pays second-highest bid (incentivizes truthful bidding).
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
