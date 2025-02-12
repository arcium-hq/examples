use arcis::prelude::*;
use crypto::*;

arcis_linker!();

#[derive(ArcisType, Copy, Clone, ArcisEncryptable)]
pub struct VoteStats {
    yes: mu64,
    no: mu64,
}

#[derive(ArcisType, Copy, Clone, ArcisEncryptable)]
pub struct UserVote {
    vote: mbool,
}

#[confidential]
pub fn vote(
    vote: [Ciphertext; 1],
    vote_public_key: PublicKey,
    vote_nonce: u128,
    vote_stats: &[Ciphertext; 2],
    vote_stats_public_key: PublicKey,
    vote_stats_nonce: u128,
) {
    let vote_cipher = RescueCipher::new_with_client(vote_public_key);
    let user_vote = vote_cipher.decrypt::<1, UserVote>(vote, vote_nonce);

    let vote_stats_cipher = RescueCipher::new_with_client(vote_stats_public_key);
    let mut vote_stats = vote_stats_cipher.decrypt::<2, VoteStats>(vote_stats, vote_stats_nonce);

    vote_stats.yes = user_vote.vote.select(vote_stats.yes + 1, vote_stats.yes);
    vote_stats.no = user_vote.vote.select(vote_stats.no, vote_stats.no + 1);
}

#[confidential]
pub fn reveal_result(
    vote_stats: &[Ciphertext; 2],
    vote_stats_public_key: PublicKey,
    vote_stats_nonce: u128,
) -> bool {
    let vote_stats_cipher = RescueCipher::new_with_client(vote_stats_public_key);
    let vote_stats = vote_stats_cipher.decrypt::<2, VoteStats>(vote_stats, vote_stats_nonce);

    vote_stats.yes.gt(vote_stats.no).reveal()
}
