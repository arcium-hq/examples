# Rock Paper Scissors

Two variants of the same game, differing in who the opponent is. In
[Player vs Player](./against-player/), both players submit client-encrypted moves
into MXE-owned game state, so neither can see the other's choice before committing;
only the outcome is revealed. In [Player vs House](./against-house/), the opponent is
the MPC cluster itself: the house move comes from in-circuit randomness, making it
provably fair — no one, including the house, knows it before the player commits.

- [Player vs Player](./against-player/) — asynchronous hidden moves from two encrypted submissions
- [Player vs House](./against-house/) — provably fair RNG opponent
