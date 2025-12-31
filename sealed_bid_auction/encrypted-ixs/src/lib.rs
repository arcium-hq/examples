use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    /// A bid submitted by a participant.
    /// Contains the bidder's Solana public key (split into two u128s) and their bid amount.
    pub struct Bid {
        /// Lower 128 bits of the bidder's Solana wallet public key
        pub bidder_lo: u128,
        /// Upper 128 bits of the bidder's Solana wallet public key
        pub bidder_hi: u128,
        /// Bid amount in lamports
        pub amount: u64,
    }

    /// Encrypted auction state maintained by the MXE.
    /// Tracks the highest and second-highest bids for both auction types.
    pub struct AuctionState {
        /// Current highest bid amount
        pub highest_bid: u64,
        /// Lower 128 bits of highest bidder's public key
        pub highest_bidder_lo: u128,
        /// Upper 128 bits of highest bidder's public key
        pub highest_bidder_hi: u128,
        /// Second highest bid amount (for Vickrey auctions)
        pub second_highest_bid: u64,
        /// Total number of bids placed
        pub bid_count: u8,
    }

    /// Result of the auction containing winner information.
    pub struct AuctionResult {
        /// Lower 128 bits of the winning bidder's public key
        pub winner_lo: u128,
        /// Upper 128 bits of the winning bidder's public key
        pub winner_hi: u128,
        /// The amount the winner must pay
        pub payment_amount: u64,
    }

    /// Initializes a new auction with empty state.
    /// The state is encrypted to the MXE so no individual participant can decrypt it.
    ///
    /// # Arguments
    /// * `mxe` - The MXE owner for the encrypted state
    ///
    /// # Returns
    /// Encrypted auction state with all values initialized to zero
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

    /// Places a bid in the auction.
    /// Compares the new bid with the current highest bid and updates state accordingly.
    /// The bid remains confidential - only the encrypted state is updated.
    ///
    /// # Arguments
    /// * `bid_ctxt` - Encrypted bid from the participant
    /// * `state_ctxt` - Current encrypted auction state
    ///
    /// # Returns
    /// Updated encrypted auction state
    #[instruction]
    pub fn place_bid(
        bid_ctxt: Enc<Shared, Bid>,
        state_ctxt: Enc<Mxe, AuctionState>,
    ) -> Enc<Mxe, AuctionState> {
        let bid = bid_ctxt.to_arcis();
        let mut state = state_ctxt.to_arcis();

        // Check if this bid is higher than the current highest
        if bid.amount > state.highest_bid {
            // Current highest becomes second highest
            state.second_highest_bid = state.highest_bid;
            // New bid becomes the highest
            state.highest_bid = bid.amount;
            state.highest_bidder_lo = bid.bidder_lo;
            state.highest_bidder_hi = bid.bidder_hi;
        } else if bid.amount > state.second_highest_bid {
            // New bid is second highest
            state.second_highest_bid = bid.amount;
        }

        state.bid_count += 1;

        state_ctxt.owner.from_arcis(state)
    }

    /// Determines the winner using first-price auction rules.
    /// The winner pays exactly what they bid.
    ///
    /// # Arguments
    /// * `state_ctxt` - Final encrypted auction state
    ///
    /// # Returns
    /// Revealed auction result with winner and payment (their bid)
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

    /// Determines the winner using Vickrey (second-price) auction rules.
    /// The winner pays the second-highest bid amount.
    /// This incentivizes truthful bidding.
    ///
    /// # Arguments
    /// * `state_ctxt` - Final encrypted auction state
    ///
    /// # Returns
    /// Revealed auction result with winner and payment (second-highest bid)
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
