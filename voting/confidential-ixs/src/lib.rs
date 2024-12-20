use arcis::prelude::*;

arcis_linker!();

#[derive(ArcisObject, Copy, Clone)]
pub struct VoteStats {
    yes: mu64,
    no: mu64,
}

#[confidential]
pub fn vote(vote: mbool, vote_stats: &mut VoteStats) {
    vote_stats.yes = vote.select(vote_stats.yes + 1, vote_stats.yes);
    vote_stats.no = vote.select(vote_stats.no, vote_stats.no + 1);
}

#[confidential]
pub fn reveal_result(vote_stats: &mut VoteStats) -> bool {
    vote_stats.yes.gt(vote_stats.no).reveal()
}
