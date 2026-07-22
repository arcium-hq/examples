//! Private voting circuits: `init_vote_stats` zeroes the tally counters, `vote` adds
//! an encrypted ballot to them, and `reveal_result` discloses only whether yes votes
//! exceed no votes. Ballots and tallies stay encrypted throughout; see README.md.

use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    /// Encrypted yes/no tallies for a poll.
    pub struct VoteStats {
        yes: u64,
        no: u64,
    }

    /// A single voter's encrypted ballot.
    pub struct UserVote {
        vote: bool,
    }

    /// Creates zeroed vote counters encrypted to the MXE. Reveals nothing.
    #[instruction]
    pub fn init_vote_stats() -> Enc<Mxe, VoteStats> {
        let vote_stats = VoteStats { yes: 0, no: 0 };
        Mxe::get().from_arcis(vote_stats)
    }

    /// Adds one encrypted ballot to the running tallies and returns them re-encrypted
    /// to the MXE. Neither the ballot nor the counts are revealed.
    #[instruction]
    pub fn vote(
        vote_ctxt: Enc<Shared, UserVote>,
        vote_stats_ctxt: Enc<Mxe, VoteStats>,
    ) -> Enc<Mxe, VoteStats> {
        let user_vote = vote_ctxt.to_arcis();
        let mut vote_stats = vote_stats_ctxt.to_arcis();

        if user_vote.vote {
            vote_stats.yes += 1;
        } else {
            vote_stats.no += 1;
        }

        vote_stats_ctxt.owner.from_arcis(vote_stats)
    }

    /// Reveals a single boolean: whether yes votes exceed no votes (ties are false).
    /// The counts themselves stay hidden.
    #[instruction]
    pub fn reveal_result(vote_stats_ctxt: Enc<Mxe, VoteStats>) -> bool {
        let vote_stats = vote_stats_ctxt.to_arcis();
        (vote_stats.yes > vote_stats.no).reveal()
    }
}
