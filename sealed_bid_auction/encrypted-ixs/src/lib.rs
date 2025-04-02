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
        highest_bidder: PublicKey,
        second_highest_bid: u128,
        second_highest_bidder: PublicKey,
    }

    pub struct Bid {
        bid: u128,
        bidder: PublicKey,
    }

    #[instruction]
    pub fn vickrey_bid(
        auction_ctxt: Enc<Mxe, VickreyAuction>,
        bid_ctxt: Enc<Mxe, Bid>,
    ) -> Enc<Mxe, VickreyAuction> {
        let mut auction = auction_ctxt.to_arcis();
        let bid = bid_ctxt.to_arcis();

        if bid.bid > auction.highest_bid {
            auction.second_highest_bid = auction.highest_bid;
            auction.second_highest_bidder = auction.highest_bidder;
            auction.highest_bid = bid.bid;
            auction.highest_bidder = bid.bidder;
        } else if bid.bid > auction.second_highest_bid {
            auction.second_highest_bid = bid.bid;
            auction.second_highest_bidder = bid.bidder;
        }

        auction_ctxt.owner.from_arcis(auction)
    }

    #[instruction]
    pub fn vickrey_reveal(auction_ctxt: Enc<Mxe, VickreyAuction>) -> (u128, PublicKey) {
        let auction = auction_ctxt.to_arcis();

        (
            auction.second_highest_bid.reveal(),
            auction.second_highest_bidder.reveal(),
        )
    }
}
