use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    pub struct VoteStats {
        yes: u64,
        no: u64,
    }

    pub struct UserVote {
        vote: bool,
    }

    #[instruction]
    pub fn init_vote_stats(mxe: Mxe) -> Enc<Mxe, VoteStats> {
        let vote_stats = VoteStats { yes: 0, no: 0 };
        mxe.from_arcis(vote_stats)
    }

    #[instruction]
    pub fn vote(
        vote: Enc<Shared, UserVote>,
        vote_stats_ctxt: Enc<Mxe, VoteStats>,
    ) -> Enc<Mxe, VoteStats> {
        let user_vote = vote.to_arcis();
        let mut vote_stats = vote_stats_ctxt.to_arcis();

        if user_vote.vote {
            vote_stats.yes += 1;
        } else {
            vote_stats.no += 1;
        }

        vote_stats_ctxt.owner.from_arcis(vote_stats)
    }

    #[instruction]
    pub fn reveal_result(vote_stats_ctxt: Enc<Mxe, VoteStats>) -> bool {
        let vote_stats = vote_stats_ctxt.to_arcis();
        (vote_stats.yes > vote_stats.no).reveal()
    }
}
