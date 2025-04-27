# Variants of Rock, Paper, Scissors

We demonstrate two variants of the classic Rock, Paper, Scissors game implemented using Arcium:

1.  [**Player vs Player (PvP)**](./against-player/): Two players securely commit to their moves (Rock, Paper, or Scissors) without revealing them to each other. The MPC protocol then computes the winner based on the committed moves, revealing only the result.
2.  [**Player vs House (PvE)**](./against-house/): One player plays against the "house". The player commits their move, while the house's move is generated randomly within the MPC computation itself. Again, only the final result (win, lose, or draw) is revealed.
