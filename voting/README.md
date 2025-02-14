# Structure of this project

**In order to build this project, cargo will require access to the arcium registry where the arcium dependencies are published to.
This is done by editing the generated `.cargo/credentials.toml` file to the root of the project with the provided token.**

This project is structured pretty similarly to how a regular Solana Anchor project is structured. The main difference lies in there being two places to write code here:

- The `programs` dir like normal
- The `confidential-ixs` dir for confidential computing instructions

When working with plaintext data, we can edit it inside our program as normal. When working with confidential data though, state transitions take place off-chain using the Arcium network as a co-processor. For this, we then always need two instructions in our program: one that gets called to initialize a confidential computation, and one that gets called when the computation is done and supplies the resulting data. Additionally, since the types and operations in a Solana program and in a confidential computing environment are a bit different, we define the operations themselves in the `confidential-ixs` dir using our Rust-based framework called Arcis. To link all of this together, we provide a few macros that take care of ensuring the correct accounts and data are passed for the specific initialization and callback functions:

```
// confidential-ixs/add_together.rs

use arcis::prelude::*;

arcis_main!();

// mu8 is a masked u8, i.e. an encrypted u8.
#[circuit]
fn add_together(x: mu8, y: mu8) -> mu8 {
    x + y
}

// programs/my_program/src/lib.rs

use anchor_lang::prelude::*;
use arcium_anchor::queue_computation;
use arcium_macros::{arcium_callback, callback_accounts, queue_computation_accounts};

declare_id!("<some ID>");

#[program]
pub mod my_program {
    use super::*;

    pub fn init_computation(_ctx: Context<InitComputation>, input: Vec<u8>) -> Result<()> {
        queue_computation(_ctx.accounts, input, vec![])?;
        Ok(())
    }

    #[arcium_callback(circuit = "add_together")]
    pub fn add_together_callback(ctx: Context<Callback>, output: Vec<u8>) -> Result<()> {
        msg!("Arcium callback invoked with output {:?}", output);
        Ok(())
    }
}

#[callback_accounts(circuit = "add_together")]
pub struct Callback<'info> {
    pub some_extra_acc: AccountInfo<'info>,
}

#[queue_computation_accounts(circuit = "add_together")]
pub struct InitComputation<'info> {
    pub some_extra_acc: AccountInfo<'info>,
}
```
