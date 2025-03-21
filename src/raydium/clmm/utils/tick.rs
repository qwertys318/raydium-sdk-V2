use std::ops::Shl;
use std::str::FromStr;
use rug::Integer;
use crate::raydium::clmm::utils::constants::{MAX_TICK, MIN_TICK};
use crate::raydium::clmm::utils::tick_array_bitmap::CheckTickArrayIsInitResult;
use crate::raydium::clmm::utils::tick_query::TickQuery;

pub const TICK_ARRAY_SIZE: usize = 60;
pub const TICK_ARRAY_BITMAP_SIZE: u16 = 512;

pub struct TickUtils {}

impl TickUtils {
    pub fn next_init_tick(tick_array_current: &carbon_raydium_clmm_decoder::accounts::tick_array_state::TickArrayState, current_tick_index: i32, tick_spacing: u16, zero_for_one: bool, t: bool) -> Option<&carbon_raydium_clmm_decoder::types::TickState> {
        let current_tick_array_start_index = TickQuery::get_array_start_index(current_tick_index, tick_spacing);
        if current_tick_array_start_index != tick_array_current.start_tick_index {
            return None;
        }
        let mut offset_in_array = ((current_tick_index - tick_array_current.start_tick_index) / tick_spacing as i32) as usize;
        if zero_for_one {
            // @TODO in originak is >= but it's breaking
            // while offset_in_array >= 0 {
            while offset_in_array > 0 {
                if tick_array_current.ticks[offset_in_array].liquidity_gross.gt(&0) {
                    return Some(&tick_array_current.ticks[offset_in_array]);
                }
                offset_in_array -= 1;
            }
        } else {
            if !t {
                offset_in_array += 1;
            }
            while offset_in_array < TICK_ARRAY_SIZE {
                if tick_array_current.ticks[offset_in_array].liquidity_gross.gt(&0) {
                    return Some(&tick_array_current.ticks[offset_in_array]);
                }
                offset_in_array += 1;
            }
        }
        None
    }
    pub fn first_initialized_tick<'a>(tick_array_current: &'a carbon_raydium_clmm_decoder::accounts::tick_array_state::TickArrayState, zero_for_one: bool) -> Result<&'a carbon_raydium_clmm_decoder::types::TickState, String> {
        if zero_for_one {
            let mut i = TICK_ARRAY_SIZE - 1;
            while i >= 0 {
                if tick_array_current.ticks[i].liquidity_gross.gt(&0) {
                    return Ok(&tick_array_current.ticks[i]);
                }
                i -= 1;
            }
        } else {
            let mut i = 0;
            while i < TICK_ARRAY_SIZE {
                if tick_array_current.ticks[i].liquidity_gross.gt(&0) {
                    return Ok(&tick_array_current.ticks[i]);
                }
                i += 1;
            }
        }
        Err(format!("firstInitializedTick check error: {} - {zero_for_one}", tick_array_current.start_tick_index))
    }
    pub fn check_tick_array_is_initialized(bitmap: &rug::Integer, tick: i32, tick_spacing: u16) -> CheckTickArrayIsInitResult {
        let multiplier = tick_spacing as usize  * TICK_ARRAY_SIZE;
        let compressed = (tick / multiplier as i32) + 512;
        let bit_pos = compressed.abs();
        CheckTickArrayIsInitResult { is_initialized: bitmap.get_bit(bit_pos as u32), start_index: (bit_pos - 512) * multiplier as i32 }
    }
    pub fn check_is_out_of_boundary(tick: i32) -> bool {
        tick < MIN_TICK || tick > MAX_TICK
    }
    pub fn get_tick_array_bit_index(tick_index: i32, tick_spacing: u16) -> i32 {
        let ticks_in_array = TickQuery::tick_count(tick_spacing);
        let mut start_index = tick_index as f64 / ticks_in_array as f64;
        if (tick_index < 0 && tick_index % ticks_in_array as i32 != 0) {
            start_index = start_index.ceil() - 1.0;
        } else {
            start_index = start_index.floor();
        }
        start_index as i32
    }
    pub fn get_tick_array_start_index_by_tick(tick_index: i32, tick_spacing: u16) -> i32 {
        Self::get_tick_array_bit_index(tick_index, tick_spacing) * TickQuery::tick_count(tick_spacing) as i32
    }
    pub fn get_initialized_tick_array_in_range(tick_array_bitmap: &[u64; 16], ex_bitmap_info: &carbon_raydium_clmm_decoder::accounts::tick_array_bitmap_extension::TickArrayBitmapExtension, tick_spacing: u16, tick_array_start_index: i32, expected_count: u8) -> Vec<i32> {
        let tick_array_offset = (tick_array_start_index as usize / (tick_spacing as usize * TICK_ARRAY_SIZE)) as i32 ;
        let mut res = TickUtils::search_low_bit_from_start(tick_array_bitmap, &ex_bitmap_info, tick_array_offset - 1, expected_count, tick_spacing);
        // println!("search_low_bit_from_start: {res:?}");
        let mut high = TickUtils::search_high_bit_from_start(tick_array_bitmap, &ex_bitmap_info, tick_array_offset, expected_count, tick_spacing);
        res.append(&mut high);
        res
    }
    pub fn search_low_bit_from_start(tick_array_bitmap: &[u64; 16], ex_bitmap_info: &carbon_raydium_clmm_decoder::accounts::tick_array_bitmap_extension::TickArrayBitmapExtension, current_tick_array_bit_start_index: i32, expected_count: u8, tick_spacing: u16) -> Vec<i32> {
        let mut current_tick_array_bit_start_index = current_tick_array_bit_start_index;
        let mut tick_array_bitmaps: Vec<[u64; 8]> = vec![];
        let mut negative_tick_array_bitmap = ex_bitmap_info.negative_tick_array_bitmap.to_vec();
        negative_tick_array_bitmap.reverse();
        tick_array_bitmaps.append(&mut negative_tick_array_bitmap);
        tick_array_bitmaps.push(tick_array_bitmap[0..8].try_into().unwrap());
        tick_array_bitmaps.push(tick_array_bitmap[8..16].try_into().unwrap());
        tick_array_bitmaps.extend(ex_bitmap_info.positive_tick_array_bitmap);
        // println!("tick_array_bitmaps: {tick_array_bitmaps:?}");
        let merged: Vec<Integer> = tick_array_bitmaps.iter().map(|e| TickUtils::merge_tick_array_bitmap(e)).collect();
        let mut res: Vec<i32> = vec![];
        while current_tick_array_bit_start_index >= -7680 {
            let array_index = ((current_tick_array_bit_start_index + 7680) as f64 / 512_f64).floor() as usize;
            let search_index = ((current_tick_array_bit_start_index + 7680) % 512) as u32;
            if merged[array_index].get_bit(search_index) {
                res.push(current_tick_array_bit_start_index);
            }
            current_tick_array_bit_start_index -= 1;
            if (res.len() == expected_count as usize) {
                break;
            }
        }
        let tick_count = TickQuery::tick_count(tick_spacing) as i32;
        res.iter().map(|e| e * tick_count).collect()
    }

    pub fn search_high_bit_from_start(tick_array_bitmap: &[u64; 16], ex_bitmap_info: &carbon_raydium_clmm_decoder::accounts::tick_array_bitmap_extension::TickArrayBitmapExtension, current_tick_array_bit_start_index: i32, expected_count: u8, tick_spacing: u16) -> Vec<i32> {
        let mut current_tick_array_bit_start_index = current_tick_array_bit_start_index;
        let mut tick_array_bitmaps: Vec<[u64; 8]> = vec![];
        let mut negative_tick_array_bitmap = ex_bitmap_info.negative_tick_array_bitmap.to_vec();
        negative_tick_array_bitmap.reverse();
        tick_array_bitmaps.append(&mut negative_tick_array_bitmap);
        tick_array_bitmaps.push(tick_array_bitmap[0..8].try_into().unwrap());
        tick_array_bitmaps.push(tick_array_bitmap[8..16].try_into().unwrap());
        tick_array_bitmaps.extend(ex_bitmap_info.positive_tick_array_bitmap);
        // println!("tick_array_bitmaps: {tick_array_bitmaps:?}");
        let merged: Vec<Integer> = tick_array_bitmaps.iter().map(|e| TickUtils::merge_tick_array_bitmap(e)).collect();
        let mut res: Vec<i32> = vec![];
        while current_tick_array_bit_start_index < 7680 {
            let array_index = ((current_tick_array_bit_start_index + 7680) as f64 / 512_f64).floor() as usize;
            let search_index = ((current_tick_array_bit_start_index + 7680) % 512) as u32;
            if merged[array_index].get_bit(search_index) {
                res.push(current_tick_array_bit_start_index);
            }
            current_tick_array_bit_start_index += 1;
            if (res.len() == expected_count as usize) {
                break;
            }
        }
        let tick_count = TickQuery::tick_count(tick_spacing) as i32;
        res.iter().map(|e| e * tick_count).collect()
    }

    pub fn merge_tick_array_bitmap(bns: &[u64]) -> Integer {
        let mut b = Integer::ZERO;
        for i in 0..bns.len() {
            let sh = Integer::from_str(&bns[i].to_string()).unwrap().shl(64 * i);
            b += sh;
        }
        b
    }
}