#[cfg(not(feature = "solana"))]
arcis_compiler::arcis_compiler!();

#[cfg(feature = "solana")]
pub fn main() {}
