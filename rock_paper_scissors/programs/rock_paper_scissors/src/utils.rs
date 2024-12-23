use anchor_lang::solana_program::keccak::hash;

pub fn get_offset(id: u32, num: u8) -> u32 {
    let mut bytes = [0u8; 5]; // 4 bytes for u32 id + 1 byte for u8 num
    bytes[..4].copy_from_slice(&id.to_be_bytes());
    bytes[4] = num;

    u32::from_be_bytes(hash(&bytes).to_bytes()[..4].try_into().unwrap())
}
