# Confidential On-Chain Voting with Arcium

This example demonstrates how to implement confidential voting on Solana using Arcium's encrypted computation framework. The system allows voters to cast their votes privately while maintaining the integrity of the voting process.

## Overview

The voting system consists of three main components:

1. Creating a poll
2. Casting a vote
3. Revealing results

### Key Features

- **Confidential Voting**: Votes are encrypted and processed in a privacy-preserving manner
- **Secure Result Revelation**: Only the poll authority can reveal the final results

## How It Works

### 1. Poll Creation

When a new poll is created:

- A unique poll account is initialized with a question and initial encrypted vote counts
- The vote counts are initialized to zero using encrypted computation
- The poll creator becomes the authority who can later reveal results

### 2. Vote Casting

The voting process uses Arcium's encrypted computation framework:

- Each vote is encrypted using the client's encryption key
- Votes are processed using homomorphic encryption, allowing addition of encrypted values
- The vote counts are updated without revealing individual votes
- A timestamp is recorded for each vote for audit purposes

### 3. Result Revelation

Only the poll authority can reveal the final results:

- The encrypted vote counts are processed to determine the winner
- The result is revealed as a boolean (true for "yes" winning, false for "no" winning)
- The process maintains privacy of individual votes while providing verifiable results

## Technical Implementation

### Encrypted Computation

The system uses Arcium's encrypted instruction framework, Arcis, with three main computations:

1. `init_vote_stats`: Initializes encrypted vote counts
2. `vote`: Processes an encrypted vote and updates the encrypted vote counts
3. `reveal_result`: Computes the final result from encrypted vote counts

### Data Structures

- `PollAccount`: Solana account structure that stores poll metadata and encrypted vote state
- Arcis Data Structures (used in encrypted computation):
  - `VoteStats`: Encrypted structure containing yes/no vote counts, processed within Arcis
  - `UserVote`: Encrypted structure for individual votes, processed within Arcis

### Privacy Guarantees

The system ensures:

- Individual votes remain private
- Vote counts are encrypted during computation
- Results are only revealed when explicitly requested by the authority
