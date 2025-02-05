use arcis::prelude::*;

arcis_linker!();

/// A Vickrey auction or sealed-bid second-price auction (SBSPA) is a type of sealed-bid auction.
/// Bidders submit written bids without knowing the bid of the other people in the auction.
/// The highest bidder wins but the price paid is the second-highest bid. This type of auction is
/// strategically similar to an English auction and gives bidders an incentive to bid their true value.
#[derive(ArcisObject)]
pub struct VickeryAuction {
    highest_bid: mu128,
    highest_bidder: mu128,
    snd_highest_bid: mu128,
}

#[derive(ArcisType)]
pub struct Bid {
    price: mu128,
    bidder: mu128,
}

#[confidential]
pub fn bid(bid: Bid, auction: &mut VickeryAuction) {
    auction = arcis!(if auction.highest_bid < bid.price {
        VickeryAuction {
            highest_bid: bid.price,
            highest_bidder: bid.bidder,
            snd_highest_bid: auction.highest_bid,
        }
    } else if auction.snd_highest_bid < bid.price {
        VickeryAuction {
            highest_bid: auction.highest_bid,
            highest_bidder: auction.highest_bidder,
            snd_highest_bid: bid.price,
        }
    } else {
        auction
    });
}

#[confidential]
pub fn sell(auction: &mut VickeryAuction) -> (u128, u128) {
    (
        auction.snd_highest_bid.reveal(),
        auction.highest_bidder.reveal(),
    )
}
