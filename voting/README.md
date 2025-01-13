# Confidential Voting Demo

This project demonstrates how to build a confidential voting system using Solana and Arcium. The system allows for private votes where:
1. Individual votes remain encrypted and confidential
2. Vote tallying happens on encrypted data
3. Only the final result (majority vote) is revealed

## How It Works

The voting system consists of two main components:

1. **Solana Program** (`programs/voting/src/lib.rs`): Handles the on-chain logic for:
   - Creating new polls
   - Submitting encrypted votes
   - Revealing final results (authorized users only)

2. **Confidential Instructions** (`confidential-ixs/src/lib.rs`): Processes the encrypted votes using:
   - Encrypted vote counting for yes/no responses
   - Secure comparison of final tallies
   - Result revelation only when authorized

## User Flow

1. **Create a Poll**
   ```bash
   # An authority creates a new poll with a question
   create-poll "Should we implement feature X?"
   ```

2. **Submit Votes**
   ```bash
   # Users submit encrypted votes (true for yes, false for no)
   vote <poll-id> true|false
   ```
   - Votes are encrypted before submission
   - Neither other voters nor the poll creator can see individual votes
   - Vote tallies are updated confidentially

3. **Reveal Results**
   ```bash
   # Only the poll authority can reveal results
   reveal-result <poll-id>
   ```
   - Returns true if majority voted yes, false otherwise
   - Individual votes remain confidential
   - Only the final boolean result is revealed

## Technical Details

The project uses Arcium's confidential computing network to process encrypted votes. When users vote:
- Votes are encrypted client-side
- The `vote()` confidential instruction processes the encrypted vote
- Vote tallies are maintained in an encrypted `VoteStats` structure
- The `reveal_result()` instruction only exposes the final boolean outcome
