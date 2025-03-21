use solana_sdk::pubkey::Pubkey;

#[derive(Debug)]
pub struct ComputeClmmPoolInfo {
    pub id: Pubkey,
    pub program_id: Pubkey,
    pub pool_state: carbon_raydium_clmm_decoder::accounts::pool_state::PoolState,
    pub ex_bitmap_info: carbon_raydium_clmm_decoder::accounts::tick_array_bitmap_extension::TickArrayBitmapExtension,
    pub amm_config: carbon_raydium_clmm_decoder::accounts::amm_config::AmmConfig,
}

impl ComputeClmmPoolInfo {
    pub fn new(id: Pubkey, program_id: Pubkey, pool_state: carbon_raydium_clmm_decoder::accounts::pool_state::PoolState, ex_bitmap_info: carbon_raydium_clmm_decoder::accounts::tick_array_bitmap_extension::TickArrayBitmapExtension, amm_config: carbon_raydium_clmm_decoder::accounts::amm_config::AmmConfig) -> Self {
        Self {
            id,
            program_id,
            pool_state,
            ex_bitmap_info,
            amm_config,
        }
    }
}