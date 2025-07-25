use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    /// Tracks the encrypted vote tallies for a poll.
    pub struct VoteStats {
        yes: u64,
        no: u64,
    }

    /// Represents a single encrypted vote.
    pub struct UserVote {
        vote: bool,
    }

    /// Initializes encrypted vote counters for a new poll.
    ///
    /// Creates a VoteStats structure with zero counts for both yes and no votes.
    /// The counters remain encrypted and can only be updated through MPC operations.
    #[instruction]
    pub fn init_vote_stats(mxe: Mxe) -> Enc<Mxe, VoteStats> {
        let vote_stats = VoteStats { yes: 0, no: 0 };
        mxe.from_arcis(vote_stats)
    }

    /// Processes an encrypted vote and updates the running tallies.
    ///
    /// Takes an individual vote and adds it to the appropriate counter (yes or no)
    /// without revealing the vote value. The updated vote statistics remain encrypted
    /// and can only be revealed by the poll authority.
    ///
    /// # Arguments
    /// * `vote_ctxt` - The encrypted vote to be counted
    /// * `vote_stats_ctxt` - Current encrypted vote tallies
    ///
    /// # Returns
    /// Updated encrypted vote statistics with the new vote included
    #[instruction]
    pub fn vote(
        vote_ctxt: Enc<Shared, UserVote>,
        vote_stats_ctxt: Enc<Mxe, VoteStats>,
    ) -> Enc<Mxe, VoteStats> {
        let user_vote = vote_ctxt.to_arcis();
        let mut vote_stats = vote_stats_ctxt.to_arcis();

        // Increment appropriate counter based on vote value
        if user_vote.vote {
            vote_stats.yes += 1;
        } else {
            vote_stats.no += 1;
        }

        vote_stats_ctxt.owner.from_arcis(vote_stats)
    }

    /// Reveals the final result of the poll by comparing vote tallies.
    ///
    /// Decrypts the vote counters and determines whether the majority voted yes or no.
    /// Only the final result (majority decision) is revealed, not the actual vote counts.
    ///
    /// # Arguments
    /// * `vote_stats_ctxt` - Encrypted vote tallies to be revealed
    ///
    /// # Returns
    /// * `true` if more people voted yes than no
    /// * `false` if more people voted no than yes (or tie)
    #[instruction]
    pub fn reveal_result(vote_stats_ctxt: Enc<Mxe, VoteStats>) -> bool {
        let vote_stats = vote_stats_ctxt.to_arcis();
        (vote_stats.yes > vote_stats.no).reveal()
    }
}
