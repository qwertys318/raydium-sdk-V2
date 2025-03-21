use std::collections::HashMap;
use std::ops::Neg;
use carbon_raydium_clmm_decoder::accounts::tick_array_state::TickArrayState;
use solana_sdk::pubkey::Pubkey;
use crate::raydium::clmm::tpe::ComputeClmmPoolInfo;
use crate::raydium::clmm::utils::constants::{MAX_TICK, MIN_TICK};
use crate::raydium::clmm::utils::math::SwapMath;
use crate::raydium::clmm::utils::pda::get_pda_tick_array_address;
use crate::raydium::clmm::utils::tick::TickUtils;
use crate::raydium::clmm::utils::tick_array_bitmap::{TickArrayBitmap, TickArrayBitmapExtensionUtils};
use crate::raydium::clmm::utils::tick_query::TickQuery;

#[derive(Debug)]
pub struct GetInputAmountAndRemainAccountsResult {
    expected_amount_in: rug::Integer,
    // @TODO remaining_accounts: Vec<Pubkey>,
    execution_price: u128,
    fee_amount: rug::Integer,
}

#[derive(Debug)]
pub struct GetFirstInitializedTickArrayResult {
    start_index: i32,
    next_account_meta: Pubkey,
}

pub struct TickRange {
    min_tick_boundary: i32,
    max_tick_boundary: i32,
}

pub struct PoolUtils {}

impl PoolUtils {
    pub fn is_overflow_default_tickarray_bitmap(tick_spacing: u16, tick_array_start_indexs: Vec<i32>) -> bool {
        let tick_range = Self::tick_range(tick_spacing);
        for tick_index in tick_array_start_indexs {
            let tick_array_start_index = TickUtils::get_tick_array_start_index_by_tick(tick_index, tick_spacing);
            if tick_array_start_index >= tick_range.max_tick_boundary || tick_array_start_index < tick_range.min_tick_boundary {
                return true;
            }
        }
        false
    }
    pub fn tick_range(tick_spacing: u16) -> TickRange {
        let mut max_tick_boundary = TickArrayBitmap::max_tick_in_tick_array_bitmap(tick_spacing) as i32;
        let mut min_tick_boundary = max_tick_boundary.neg();
        if max_tick_boundary > MAX_TICK {
            max_tick_boundary = TickQuery::get_array_start_index(MAX_TICK, tick_spacing) + TickQuery::tick_count(tick_spacing) as i32;
        }
        if min_tick_boundary < MIN_TICK {
            min_tick_boundary = TickQuery::get_array_start_index(MIN_TICK, tick_spacing);
        }
        TickRange { min_tick_boundary, max_tick_boundary }
    }
    pub fn next_initialized_tick_array_start_index(
        tick_current: i32,
        tick_spacing: u16,
        tick_array_bitmap: &[u64; 16],
        ex_bitmap_info: &carbon_raydium_clmm_decoder::accounts::tick_array_bitmap_extension::TickArrayBitmapExtension,
        zero_for_one: bool,
    ) -> Result<Option<i32>, String> {
        let mut last_tick_array_start_index = TickQuery::get_array_start_index(tick_current, tick_spacing);
        loop {
            let next_initialized_tick_array_start_index = TickArrayBitmap::next_initialized_tick_array_start_index(
                &TickUtils::merge_tick_array_bitmap(tick_array_bitmap.as_slice()),
                last_tick_array_start_index,
                tick_spacing,
                zero_for_one,
            )?;
            if next_initialized_tick_array_start_index.is_init {
                return Ok(Some(next_initialized_tick_array_start_index.tick_index));
            }
            last_tick_array_start_index = next_initialized_tick_array_start_index.tick_index;

            let next_initialized_tick_array_from_one_bitmap = TickArrayBitmapExtensionUtils::next_initialized_tick_array_from_one_bitmap(
                last_tick_array_start_index,
                tick_spacing,
                zero_for_one,
                &ex_bitmap_info,
            )?;
            if next_initialized_tick_array_from_one_bitmap.is_init {
                return Ok(Some(next_initialized_tick_array_from_one_bitmap.tick_index));
            }
            last_tick_array_start_index = next_initialized_tick_array_from_one_bitmap.tick_index;
            if last_tick_array_start_index < MIN_TICK || last_tick_array_start_index > MAX_TICK {
                return Ok(None);
            }
        }
    }
    pub fn get_first_initialized_tick_array(
        pool_info: &ComputeClmmPoolInfo,
        zero_for_one: bool,
    ) -> Result<GetFirstInitializedTickArrayResult, String> {
        let array_start_index = if Self::is_overflow_default_tickarray_bitmap(pool_info.pool_state.tick_spacing, vec![pool_info.pool_state.tick_current]) {
            TickArrayBitmapExtensionUtils::check_tick_array_is_init(
                TickQuery::get_array_start_index(pool_info.pool_state.tick_current, pool_info.pool_state.tick_spacing),
                pool_info.pool_state.tick_spacing,
                &pool_info.ex_bitmap_info,
            )?
        } else {
            TickUtils::check_tick_array_is_initialized(
                &TickUtils::merge_tick_array_bitmap(pool_info.pool_state.tick_array_bitmap.as_slice()),
                pool_info.pool_state.tick_current,
                pool_info.pool_state.tick_spacing,
            )
        };
        if array_start_index.is_initialized {
            let (next_account_meta, _) = get_pda_tick_array_address(&pool_info.program_id, &pool_info.id, &(array_start_index.start_index as i32));
            Ok(GetFirstInitializedTickArrayResult { start_index: array_start_index.start_index, next_account_meta })
        } else {
            match Self::next_initialized_tick_array_start_index(
                pool_info.pool_state.tick_current,
                pool_info.pool_state.tick_spacing,
                &pool_info.pool_state.tick_array_bitmap,
                &pool_info.ex_bitmap_info,
                // TickQuery::get_array_start_index(pool_info.pool_state.tick_current, pool_info.pool_state.tick_spacing),
                zero_for_one,
            )? {
                None => Err("Neither initialized nor exist.".to_string()),
                Some(next_init_start_index) => {
                    let (next_account_meta, _) = get_pda_tick_array_address(&pool_info.program_id, &pool_info.id, &next_init_start_index);
                    Ok(GetFirstInitializedTickArrayResult { start_index: next_init_start_index, next_account_meta })
                }
            }
        }
    }
    pub fn get_input_amount_and_remain_accounts(
        pool_info: &ComputeClmmPoolInfo,
        tick_array_cache: &HashMap<i32, TickArrayState>,
        output_token_mint: &Pubkey,
        output_amount: rug::Integer,
        // @TODO price_limit: Decimal
    ) -> Result<GetInputAmountAndRemainAccountsResult, String> {
        let zero_for_one = output_token_mint == &pool_info.pool_state.token_mint1;
        // let mut all_needed_accounts: Vec<Pubkey> = vec![];
        let first_tick_array_start_index = Self::get_first_initialized_tick_array(pool_info, zero_for_one)?;
        // println!("first_tick_array_start_index {first_tick_array_start_index:?}");
        let swap_compute = SwapMath::swap_compute(
            &pool_info.program_id,
            &pool_info.id,
            tick_array_cache,
            &pool_info.pool_state.tick_array_bitmap,
            &pool_info.ex_bitmap_info,
            zero_for_one,
            pool_info.amm_config.trade_fee_rate,
            pool_info.pool_state.liquidity,
            pool_info.pool_state.tick_current,
            pool_info.pool_state.tick_spacing,
            pool_info.pool_state.sqrt_price_x64,
            output_amount.neg(),
            first_tick_array_start_index.start_index,
            None,
            false,
        )?;
        // println!("swap_compute {swap_compute:?}");
        Ok(GetInputAmountAndRemainAccountsResult {
            expected_amount_in: swap_compute.amount_calculated,
            // @TODO remaining_accounts:,
            execution_price: swap_compute.sqrt_price_x64,
            fee_amount: swap_compute.fee_amount,
        }
        )
    }
}