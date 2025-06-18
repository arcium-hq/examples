use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    /// A Vickrey auction or sealed-bid second-price auction (SBSPA) is a type of sealed-bid auction.
    /// Bidders submit written bids without knowing the bid of the other people in the auction.
    /// The highest bidder wins but the price paid is the second-highest bid. This type of auction is
    /// strategically similar to an English auction and gives bidders an incentive to bid their true value.
    pub struct VickreyAuction {
        highest_bid: u128,
        highest_bidder_pubkey_serialized: SerializedSolanaPublicKey,
        second_highest_bid: u128,
        second_highest_bidder_pubkey_serialized: SerializedSolanaPublicKey,
    }

    pub struct Bid {
        value: u128,
        bidder_pubkey_serialized: SerializedSolanaPublicKey,
    }

    #[instruction]
    pub fn setup_vickrey_auction() -> Enc<Mxe, VickreyAuction> {
        let auction = VickreyAuction {
            highest_bid: 0,
            highest_bidder_pubkey_serialized: SerializedSolanaPublicKey { lo: 0, hi: 0 },
            second_highest_bid: 0,
            second_highest_bidder_pubkey_serialized: SerializedSolanaPublicKey { lo: 0, hi: 0 },
        };

        let mxe = Mxe::get();

        mxe.from_arcis(auction)
    }

    #[instruction]
    pub fn vickrey_auction_place_bid(
        bid_ctxt: Enc<Shared, Bid>,
        auction_ctxt: Enc<Mxe, VickreyAuction>,
    ) -> Enc<Mxe, VickreyAuction> {
        let mut auction = auction_ctxt.to_arcis();
        let bid = bid_ctxt.to_arcis();

        if bid.value > auction.highest_bid {
            // If the bid is higher than the highest bid, update the second highest bid and bidder to the previous highest bid and bidder
            auction.second_highest_bid = auction.highest_bid;
            auction.second_highest_bidder_pubkey_serialized =
                auction.highest_bidder_pubkey_serialized;

            // Update the highest bid and bidder to the new bid and bidder
            auction.highest_bid = bid.value;
            auction.highest_bidder_pubkey_serialized = bid.bidder_pubkey_serialized;
        } else if bid.value > auction.second_highest_bid {
            // If the bid is higher than the second highest bid, update the second highest bid and bidder to the new bid and bidder
            auction.second_highest_bid = bid.value;
            auction.second_highest_bidder_pubkey_serialized = bid.bidder_pubkey_serialized;
        }

        auction_ctxt.owner.from_arcis(auction)
    }

    #[instruction]
    pub fn vickrey_auction_reveal_result(
        auction_ctxt: Enc<Mxe, VickreyAuction>,
    ) -> (SerializedSolanaPublicKey, u128) {
        let auction = auction_ctxt.to_arcis();

        // Highest bidder wins the auction and pays the second highest bid
        (
            auction.highest_bidder_pubkey_serialized.reveal(),
            auction.second_highest_bid.reveal(),
        )
    }
}
