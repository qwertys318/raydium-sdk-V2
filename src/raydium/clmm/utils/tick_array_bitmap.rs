use std::ops::{Neg, Shl, Shr};
use carbon_raydium_clmm_decoder::accounts::tick_array_bitmap_extension::TickArrayBitmapExtension;
use rug::Complete;
use crate::raydium::clmm::utils::constants::{MAX_TICK, MIN_TICK};
use crate::raydium::clmm::utils::tick::{TICK_ARRAY_BITMAP_SIZE, TICK_ARRAY_SIZE, TickUtils};
use crate::raydium::clmm::utils::tick_query::TickQuery;
use crate::raydium::clmm::utils::util::{is_zero, leading_zeros, least_significant_bit, most_significant_bit, trailing_zeros};

pub struct TickArrayBitmap {}

pub struct TickArrayBitmapExtensionUtils {}

pub struct NextTickArray {
    pub is_init: bool,
    pub tick_index: i32,
}

pub struct GetBitmapTickBoundary {
    min: i32,
    max: i32,
}

impl TickArrayBitmap {
    pub fn get_bitmap_tick_boundary(tick_array_start_index: i32, tick_spacing: u16) -> GetBitmapTickBoundary {
        let ticks_in_one_bitmap = Self::max_tick_in_tick_array_bitmap(tick_spacing) as i32;
        let tick_array_start_index_abs = tick_array_start_index.abs();
        let mut m = (tick_array_start_index_abs as f32 / ticks_in_one_bitmap as f32).floor() as i32;
        if tick_array_start_index < 0 && tick_array_start_index_abs % ticks_in_one_bitmap != 0 {
            m += 1;
        }
        let min = ticks_in_one_bitmap * m;
        if tick_array_start_index < 0 {
            let min_neg = min.neg();
            GetBitmapTickBoundary { min: min_neg, max: min_neg + ticks_in_one_bitmap }
        } else {
            GetBitmapTickBoundary { min, max: min + ticks_in_one_bitmap }
        }
    }
    pub fn max_tick_in_tick_array_bitmap(tick_spacing: u16) -> u32 {
        tick_spacing as u32 * TICK_ARRAY_SIZE as u32 * TICK_ARRAY_BITMAP_SIZE as u32
    }
    pub fn next_initialized_tick_array_start_index(bitmap: &rug::Integer, last_tick_array_start_index: i32, tick_spacing: u16, zero_for_one: bool) -> Result<NextTickArray, String> {
        if !TickQuery::check_is_valid_start_index(last_tick_array_start_index, tick_spacing) {
            return Err("nextInitializedTickArrayStartIndex check error".to_string());
        }
        let tick_boundary = Self::max_tick_in_tick_array_bitmap(tick_spacing) as i32;
        let next_tick_array_start_index = if zero_for_one {
            last_tick_array_start_index - TickQuery::tick_count(tick_spacing) as i32
        } else {
            last_tick_array_start_index + TickQuery::tick_count(tick_spacing) as i32
        };
        if next_tick_array_start_index < -tick_boundary || next_tick_array_start_index >= tick_boundary {
            return Ok(NextTickArray { is_init: false, tick_index: last_tick_array_start_index });
        }
        let multiplier = (tick_spacing as usize * TICK_ARRAY_SIZE) as i32;
        let mut compressed = next_tick_array_start_index / multiplier + 512;
        if next_tick_array_start_index < 0 && next_tick_array_start_index % multiplier != 0 {
            compressed -= 1;
        }
        let bit_pos = compressed.abs();
        return if zero_for_one {
            let offset_bitmap = bitmap.shl(1024 - bit_pos - 1).complete();
            match most_significant_bit(1024, &offset_bitmap) {
                None => {
                    Ok(NextTickArray { is_init: false, tick_index: -tick_boundary })
                }
                Some(next_bit) => {
                    let next_array_start_index = (bit_pos - next_bit as i32 - 512) * multiplier;
                    Ok(NextTickArray { is_init: true, tick_index: next_array_start_index })
                }
            }
        } else {
            let offset_bitmap = bitmap.shr(bit_pos).complete();
            match least_significant_bit(1024, &offset_bitmap) {
                None => {
                    Ok(NextTickArray { is_init: false, tick_index: tick_boundary - TickQuery::tick_count(tick_spacing) as i32 })
                }
                Some(next_bit) => {
                    let next_array_start_index = (bit_pos + next_bit as i32 - 512) * multiplier;
                    Ok(NextTickArray { is_init: true, tick_index: next_array_start_index })
                }
            }
        }
    }
}

pub struct CheckTickArrayIsInitResult {
    pub is_initialized: bool,
    pub start_index: i32,
}

pub struct GetBitmapResult {
    offset: u32,
    tick_array_bitmap: [u64; 8],
}

pub struct ExtensionTickBoundaryResult {
    positive_tick_boundary: i32,
    negative_tick_boundary: i32,
}

impl TickArrayBitmapExtensionUtils {
    pub fn next_initialized_tick_array_in_bitmap(
        tick_array_bitmap: [u64; 8],
        next_tick_array_start_index: i32,
        tick_spacing: u16,
        zero_for_one: bool,
    ) -> NextTickArray {
        let bitmap_tick_boundary = TickArrayBitmap::get_bitmap_tick_boundary(next_tick_array_start_index, tick_spacing);
        let tick_array_offset_in_bitmap = Self::tick_array_offset_in_bitmap(next_tick_array_start_index, tick_spacing);
        if zero_for_one {
            let offset_bitmap = TickUtils::merge_tick_array_bitmap(tick_array_bitmap.as_slice()).shl(TICK_ARRAY_BITMAP_SIZE as u32 - 1 - tick_array_offset_in_bitmap);
            return if is_zero(512, &offset_bitmap) {
                NextTickArray { is_init: false, tick_index: bitmap_tick_boundary.min }
            } else {
                let next_bit = leading_zeros(512, &offset_bitmap);
                let next_array_start_index = next_tick_array_start_index - next_bit as i32 * TickQuery::tick_count(tick_spacing) as i32;
                NextTickArray { is_init: true, tick_index: next_array_start_index }
            };
        } else {
            let offset_bitmap = TickUtils::merge_tick_array_bitmap(tick_array_bitmap.as_slice()).shr(tick_array_offset_in_bitmap);
            return if is_zero(512, &offset_bitmap) {
                NextTickArray { is_init: false, tick_index: bitmap_tick_boundary.max - TickQuery::tick_count(tick_spacing) as i32 }
            } else {
                let next_bit = trailing_zeros(512, &offset_bitmap);
                let next_array_start_index = next_tick_array_start_index + next_bit as i32 * TickQuery::tick_count(tick_spacing) as i32;
                NextTickArray { is_init: true, tick_index: next_array_start_index }
            };
        }
    }
    pub fn next_initialized_tick_array_from_one_bitmap(
        last_tick_array_start_index: i32,
        tick_spacing: u16,
        zero_for_one: bool,
        tick_array_bitmap_extension: &carbon_raydium_clmm_decoder::accounts::tick_array_bitmap_extension::TickArrayBitmapExtension,
    ) -> Result<NextTickArray, String> {
        let multiplier = TickQuery::tick_count(tick_spacing) as i32;
        let next_tick_array_start_index = if zero_for_one {
            last_tick_array_start_index - multiplier as i32
        } else {
            last_tick_array_start_index + multiplier
        };
        let tick_array_bitmap = Self::get_bitmap(next_tick_array_start_index, tick_spacing, tick_array_bitmap_extension)?;
        Ok(Self::next_initialized_tick_array_in_bitmap(tick_array_bitmap.tick_array_bitmap, next_tick_array_start_index, tick_spacing, zero_for_one))
    }
    pub fn extension_tick_boundary(tick_spacing: u16) -> Result<ExtensionTickBoundaryResult, String> {
        let positive_tick_boundary = TickArrayBitmap::max_tick_in_tick_array_bitmap(tick_spacing) as i32;
        let negative_tick_boundary = positive_tick_boundary.neg();
        if MAX_TICK <= positive_tick_boundary {
            return Err(format!("extensionTickBoundary check error: {MAX_TICK}, {positive_tick_boundary}"));
        }
        if negative_tick_boundary <= MIN_TICK {
            return Err(format!("extensionTickBoundary check error: {negative_tick_boundary}, {MIN_TICK}"));
        }
        Ok(ExtensionTickBoundaryResult { positive_tick_boundary, negative_tick_boundary })
    }
    pub fn check_extension_boundary(tick_index: i32, tick_spacing: u16) -> Result<(), String> {
        let extension_tick_boundary = Self::extension_tick_boundary(tick_spacing)?;
        if tick_index >= extension_tick_boundary.negative_tick_boundary && tick_index < extension_tick_boundary.positive_tick_boundary {
            return Err("checkExtensionBoundary -> InvalidTickArrayBoundary".to_string());
        }
        Ok(())
    }
    pub fn get_bitmap_offset(tick_index: i32, tick_spacing: u16) -> Result<u32, String> {
        if !TickQuery::check_is_valid_start_index(tick_index, tick_spacing) {
            return Err("No enough initialized tickArray".to_string());
        }
        Self::check_extension_boundary(tick_index, tick_spacing)?;

        let ticks_in_one_bitmap = TickArrayBitmap::max_tick_in_tick_array_bitmap(tick_spacing);
        let tick_index_abs = tick_index.abs() as u32;
        // floor
        let mut offset = (tick_index_abs / ticks_in_one_bitmap) - 1;
        if tick_index < 0 && tick_index_abs % ticks_in_one_bitmap == 0 {
            offset -= 1;
        }
        Ok(offset)
    }
    pub fn get_bitmap(tick_index: i32, tick_spacing: u16, tick_array_bitmap_extension: &TickArrayBitmapExtension) -> Result<GetBitmapResult, String> {
        let offset = Self::get_bitmap_offset(tick_index, tick_spacing)?;
        let tick_array_bitmap = if tick_index < 0 {
            tick_array_bitmap_extension.negative_tick_array_bitmap[offset as usize]
        } else {
            tick_array_bitmap_extension.positive_tick_array_bitmap[offset as usize]
        };
        Ok(GetBitmapResult { offset, tick_array_bitmap })
    }
    pub fn check_tick_array_is_init(tick_array_start_index: i32, tick_spacing: u16, tick_array_bitmap_extension: &TickArrayBitmapExtension) -> Result<CheckTickArrayIsInitResult, String> {
        let tick_array_bitmap = Self::get_bitmap(tick_array_start_index, tick_spacing, tick_array_bitmap_extension)?;
        let tick_array_offset_in_bitmap = Self::tick_array_offset_in_bitmap(tick_array_start_index, tick_spacing);
        let is_initialized = TickUtils::merge_tick_array_bitmap(&tick_array_bitmap.tick_array_bitmap).get_bit(tick_array_offset_in_bitmap);
        Ok(CheckTickArrayIsInitResult { is_initialized, start_index: tick_array_start_index })
    }
    pub fn tick_array_offset_in_bitmap(tick_array_start_index: i32, tick_spacing: u16) -> u32 {
        let m = tick_array_start_index.abs() as u32 % TickArrayBitmap::max_tick_in_tick_array_bitmap(tick_spacing);
        // floor
        let mut tick_array_offset_in_bitmap = m / TickQuery::tick_count(tick_spacing);
        if tick_array_start_index < 0 && m != 0 {
            tick_array_offset_in_bitmap = TICK_ARRAY_BITMAP_SIZE as u32 - tick_array_offset_in_bitmap;
        }
        tick_array_offset_in_bitmap
    }
}