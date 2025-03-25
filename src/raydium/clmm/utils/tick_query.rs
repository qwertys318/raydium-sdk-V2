use crate::raydium::clmm::utils::constants::{MAX_TICK, MIN_TICK};
use crate::raydium::clmm::utils::tick::{TICK_ARRAY_SIZE, TickUtils};

pub struct TickQuery {}

impl TickQuery {
    pub fn tick_count(tick_spacing: u16) -> u32 {
        (TICK_ARRAY_SIZE * tick_spacing as usize) as u32
    }
    pub fn get_array_start_index(tick_index: i32, tick_spacing: u16) -> i32 {
        let ticks_in_array = Self::tick_count(tick_spacing);
        let start = (tick_index as f32 / ticks_in_array as f32).floor() as i32;
        start * ticks_in_array as i32
    }
    pub fn check_is_valid_start_index(tick_index: i32, tick_spacing: u16) -> bool {
        if TickUtils::check_is_out_of_boundary(tick_index) {
            if tick_index > MAX_TICK {
                return false;
            }
            let min_start_index = TickUtils::get_tick_array_start_index_by_tick(MIN_TICK, tick_spacing);
            return tick_index == min_start_index;
        }
        tick_index % Self::tick_count(tick_spacing) as i32 == 0
    }
    pub fn next_initialized_tick_array(tick_index: i32, tick_spacing: u16, zero_for_one: bool, tick_array_bitmap: &[u64; 16], tick_array_bitmap_extension: &carbon_raydium_clmm_decoder::accounts::tick_array_bitmap_extension::TickArrayBitmapExtension)->Option<i32> {
        let current_offset = (tick_index as f32 / TickQuery::tick_count(tick_spacing) as f32).floor() as i32;
        let res = if zero_for_one {
            TickUtils::search_low_bit_from_start(tick_array_bitmap, tick_array_bitmap_extension, current_offset - 1, 1, tick_spacing)
        } else {
            TickUtils::search_high_bit_from_start(tick_array_bitmap, tick_array_bitmap_extension, current_offset + 1, 1, tick_spacing)
        };
        if res.len() > 0 {
            Some(res[0])
        } else {
            None
        }
    }
}