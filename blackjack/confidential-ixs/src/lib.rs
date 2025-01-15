use arcis::prelude::*;

arcis_linker!();

#[confidential]
pub fn add_together(x: mu8, y: mu8) -> u8 {
    (x + y).reveal()
}
