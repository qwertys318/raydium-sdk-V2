use solana_sdk::pubkey::Pubkey;

pub const POOL_TICK_ARRAY_BITMAP_SEED: &str = "pool_tick_array_bitmap_extension";
pub const TICK_ARRAY_SEED: &str = "tick_array";

pub fn get_pda_ex_bitmap_account(program_id: &Pubkey, pool_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&POOL_TICK_ARRAY_BITMAP_SEED.as_bytes(), &pool_id.to_bytes().as_slice()], &program_id)
}

pub fn get_pda_tick_array_address(program_id: &Pubkey, pool_id: &Pubkey, start_index: &i32) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&TICK_ARRAY_SEED.as_bytes(), &pool_id.to_bytes().as_slice(), start_index.to_be_bytes().as_slice()], &program_id)
}