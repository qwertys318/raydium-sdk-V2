use std::cmp::min;
use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Neg, Shl, Shr, Sub};
use std::str::FromStr;
use carbon_raydium_clmm_decoder::types::TickState;
use rug::{Complete, Integer};
use solana_sdk::pubkey::Pubkey;
use crate::raydium::clmm::utils::constants::{BIT_PRECISION, FEE_RATE_DENOMINATOR, LOG_B_2_X32, LOG_B_P_ERR_MARGIN_LOWER_X64, LOG_B_P_ERR_MARGIN_UPPER_X64, MAX_SQRT_PRICE_X64, MAX_SQRT_PRICE_X64_SUB_ONE, MAX_TICK, MIN_SQRT_PRICE_X64, MIN_SQRT_PRICE_X64_ADD_ONE, MIN_TICK, Q64, U64_RESOLUTION};
use crate::raydium::clmm::utils::pda::get_pda_tick_array_address;
use crate::raydium::clmm::utils::pool::PoolUtils;
use crate::raydium::clmm::utils::tick::TickUtils;
use crate::raydium::clmm::utils::tick_query::TickQuery;

pub struct SwapMath {}

pub struct SqrtPriceMath {}

pub struct MathUtil {}

pub struct LiquidityMath {}

#[derive(Debug)]
pub struct SwapComputeResult {
    pub all_trade: bool,
    pub amount_specified_remaining: rug::Integer,
    pub amount_calculated: rug::Integer,
    pub fee_amount: rug::Integer,
    pub sqrt_price_x64: u128,
    pub liquidity: u128,
    pub tick_current: i32,
    pub accounts: Vec<Pubkey>,
}

struct SwapComputeState {
    amount_specified_remaining: rug::Integer,
    amount_calculated: rug::Integer,
    sqrt_price_x64: u128,
    tick: i32,
    accounts: Vec<Pubkey>,
    liquidity: u128,
    fee_amount: rug::Integer,
}

struct StepComputations {
    sqrt_price_start_x64: u128,
    tick_next: i32,
    initialized: bool,
    sqrt_price_next_x64: u128,
    amount_in: rug::Integer,
    amount_out: rug::Integer,
    fee_amount: rug::Integer,
}

struct SwapStepComputeResult {
    sqrt_price_x64: u128,
    amount_in: rug::Integer,
    amount_out: rug::Integer,
    fee_amount: rug::Integer,
}

impl SwapMath {
    pub fn swap_compute(
        program_id: &Pubkey,
        pool_id: &Pubkey,
        tick_array_cache: &HashMap<i32, carbon_raydium_clmm_decoder::accounts::tick_array_state::TickArrayState>,
        tick_array_bitmap: &[u64; 16],
        tick_array_bitmap_extension: &carbon_raydium_clmm_decoder::accounts::tick_array_bitmap_extension::TickArrayBitmapExtension,
        zero_for_one: bool,
        fee: u32,
        liquidity: u128,
        current_tick: i32,
        tick_spacing: u16,
        current_sqrt_price_x64: u128,
        amount_specified: rug::Integer,
        last_saved_tick_array_start_index: i32,
        sqrt_price_limit_x64: Option<u128>,
        catch_liquidity_insufficient: bool,
    ) -> Result<SwapComputeResult, String> {
        let mut last_saved_tick_array_start_index = last_saved_tick_array_start_index;
        if amount_specified.is_zero() {
            return Err("amountSpecified must not be 0".to_string());
        }
        let sqrt_price_limit_x64 = match sqrt_price_limit_x64 {
            None => {
                if zero_for_one {
                    MIN_SQRT_PRICE_X64_ADD_ONE
                } else {
                    MAX_SQRT_PRICE_X64_SUB_ONE
                }
            }
            Some(x) => x
        };
        if zero_for_one {
            if sqrt_price_limit_x64.lt(&MIN_SQRT_PRICE_X64) {
                return Err("sqrt_price_x64 must greater than MIN_SQRT_PRICE_X64".to_string());
            }
            if sqrt_price_limit_x64.ge(&current_sqrt_price_x64) {
                return Err("sqrt_price_x64 must smaller than current".to_string());
            }
        } else {
            if sqrt_price_limit_x64.gt(&MAX_SQRT_PRICE_X64) {
                return Err("sqrt_price_x64 must smaller than MAX_SQRT_PRICE_X64".to_string());
            }
            if sqrt_price_limit_x64.le(&current_sqrt_price_x64) {
                return Err("sqrt_price_x64 must greater than current".to_string());
            }
        }
        let base_input = amount_specified.gt(&0);
        let tick = if current_tick > last_saved_tick_array_start_index {
            min(last_saved_tick_array_start_index + TickQuery::tick_count(tick_spacing) as i32, current_tick)
        } else {
            last_saved_tick_array_start_index
        };
        let mut state = SwapComputeState {
            amount_specified_remaining: amount_specified,
            amount_calculated: rug::Integer::ZERO,
            sqrt_price_x64: current_sqrt_price_x64,
            tick: tick as i32,
            accounts: vec![],
            liquidity,
            fee_amount: rug::Integer::ZERO,
        };
        let mut tick_array_start_index = last_saved_tick_array_start_index;
        let mut tick_array_current = tick_array_cache.get(&last_saved_tick_array_start_index).unwrap();
        let mut loop_count = 0;
        let mut t = !zero_for_one && tick_array_current.start_tick_index == state.tick;
        while !state.amount_specified_remaining.is_zero() && !state.sqrt_price_x64.eq(&sqrt_price_limit_x64) {
            //@TODO
            let mut step = StepComputations {
                sqrt_price_start_x64: state.sqrt_price_x64.clone(),
                tick_next: 0,
                initialized: false,
                sqrt_price_next_x64: 0,
                amount_in: Default::default(),
                amount_out: Default::default(),
                fee_amount: Default::default(),
            };
            step.sqrt_price_start_x64 = state.sqrt_price_x64;
            let mut tick_array_address: Option<Pubkey> = None;
            let mut next_init_tick_opt = TickUtils::next_init_tick(tick_array_current, state.tick, tick_spacing, zero_for_one, t);
            if next_init_tick_opt.is_none() || next_init_tick_opt.as_ref().unwrap().liquidity_gross.gt(&0) {

                //@TODO remove later if ok
                let tmp_array_start_index = TickQuery::get_array_start_index(state.tick, tick_spacing);
                if tmp_array_start_index != tick_array_start_index {
                    return Err("TODO tmp_array_start_index != tick_array_start_index ?!?!?".to_string());
                }

                match PoolUtils::next_initialized_tick_array_start_index(state.tick, tick_spacing, tick_array_bitmap, tick_array_bitmap_extension, zero_for_one)? {
                    None => {
                        if catch_liquidity_insufficient {
                            return Ok(SwapComputeResult {
                                all_trade: false,
                                amount_specified_remaining: state.amount_specified_remaining,
                                amount_calculated: state.amount_calculated,
                                fee_amount: state.fee_amount,
                                sqrt_price_x64: state.sqrt_price_x64,
                                liquidity: state.liquidity,
                                tick_current: state.tick,
                                accounts: state.accounts,
                            });
                        }
                        return Err("swapCompute LiquidityInsufficient".to_string());
                    }
                    Some(next_init_tick_array_index) => {
                        tick_array_start_index = next_init_tick_array_index;
                        let expected_next_tick_array_address = get_pda_tick_array_address(program_id, pool_id, &tick_array_start_index).0;
                        tick_array_address = Some(expected_next_tick_array_address);
                        tick_array_current = tick_array_cache.get(&tick_array_start_index).unwrap();
                        match TickUtils::first_initialized_tick(tick_array_current, zero_for_one) {
                            Ok(tick) => next_init_tick_opt = Some(tick),
                            Err(e) => return Err(format!("not found next tick info: {e}"))
                        }
                    }
                }
            }
            let next_init_tick = next_init_tick_opt.unwrap();
            step.tick_next = next_init_tick.tick;
            step.initialized = next_init_tick.liquidity_gross.gt(&0);
            if last_saved_tick_array_start_index != tick_array_start_index && tick_array_address.is_some() {
                state.accounts.push(tick_array_address.unwrap());
                last_saved_tick_array_start_index = tick_array_start_index;
            }
            if step.tick_next < MIN_TICK {
                step.tick_next = MIN_TICK;
            } else if step.tick_next > MAX_TICK {
                step.tick_next = MAX_TICK;
            }
            step.sqrt_price_next_x64 = SqrtPriceMath::get_sqrt_price_x64_from_tick(step.tick_next)?;
            let target_price = if (zero_for_one && step.sqrt_price_next_x64.lt(&sqrt_price_limit_x64)) || (!zero_for_one && step.sqrt_price_next_x64.gt(&sqrt_price_limit_x64)) {
                sqrt_price_limit_x64
            } else {
                step.sqrt_price_next_x64
            };
            let swap_step_compute = Self::swap_step_compute(state.sqrt_price_x64, target_price, state.liquidity, &state.amount_specified_remaining, fee, zero_for_one)?;
            state.sqrt_price_x64 = swap_step_compute.sqrt_price_x64;
            step.amount_in = swap_step_compute.amount_in;
            step.amount_out = swap_step_compute.amount_out;
            step.fee_amount = swap_step_compute.fee_amount;

            state.fee_amount = state.fee_amount + &step.fee_amount;

            if base_input {
                state.amount_specified_remaining = state.amount_specified_remaining.sub(step.amount_in.add(step.fee_amount));
                state.amount_calculated = state.amount_calculated.sub(step.amount_out);
            } else {
                state.amount_specified_remaining = state.amount_specified_remaining.add(step.amount_out);
                state.amount_calculated = state.amount_calculated.add(step.amount_in.add(step.fee_amount));
            }
            if state.sqrt_price_x64.eq(&step.sqrt_price_next_x64) {
                if step.initialized {
                    let mut liquidity_net = next_init_tick.liquidity_net;
                    if zero_for_one {
                        liquidity_net = liquidity_net.neg();
                    }
                    state.liquidity += liquidity_net as u128;
                }
                t = step.tick_next != state.tick && !zero_for_one && tick_array_current.start_tick_index == step.tick_next;
                state.tick = if zero_for_one {
                    step.tick_next - 1
                } else {
                    step.tick_next
                }
            } else if state.sqrt_price_x64 != step.sqrt_price_start_x64 {
                let tt = SqrtPriceMath::get_tick_from_sqrt_price_x64(state.sqrt_price_x64)?;
                t = tt != state.tick && !zero_for_one && tick_array_current.start_tick_index == tt;
                state.tick = tt;
            }
            loop_count += 1;
        }
        if let Some(next_start_index) = TickQuery::next_initialized_tick_array(state.tick, tick_spacing, zero_for_one, tick_array_bitmap, tick_array_bitmap_extension) {
            if last_saved_tick_array_start_index != next_start_index {
                state.accounts.push(get_pda_tick_array_address(program_id, pool_id, &next_start_index).0);
            }
        }
        Ok(SwapComputeResult {
            all_trade: true,
            amount_specified_remaining: rug::Integer::ZERO,
            amount_calculated: state.amount_calculated,
            fee_amount: state.fee_amount,
            sqrt_price_x64: state.sqrt_price_x64,
            liquidity: state.liquidity,
            tick_current: state.tick,
            accounts: state.accounts,
        })
    }
    fn swap_step_compute(sqrt_price_x64_current: u128, sqrt_price_x64_target: u128, liquidity: u128, amount_remaining: &rug::Integer, fee_rate: u32, zero_for_one: bool) -> Result<SwapStepComputeResult, String> {
        let mut amount_in: Option<rug::Integer> = None;
        let mut amount_out: Option<rug::Integer> = None;
        let base_input = amount_remaining.gt(&rug::Integer::ZERO);
        let amount_remaining_neg = amount_remaining.neg().complete();
        let sqrt_price_x64_next = if base_input {
            let amount_remaining_subtract_fee = MathUtil::mul_div_floor(amount_remaining, &rug::Integer::from_str(&(FEE_RATE_DENOMINATOR - fee_rate).to_string()).unwrap(), &rug::Integer::from_str(&FEE_RATE_DENOMINATOR.to_string()).unwrap())?;
            amount_in = Some(if zero_for_one {
                LiquidityMath::get_token_amount_a_from_liquidity(sqrt_price_x64_target, sqrt_price_x64_current, liquidity, true)?
            } else {
                LiquidityMath::get_token_amount_b_from_liquidity(sqrt_price_x64_current, sqrt_price_x64_target, liquidity, true)?
            });
            let sqrt_price_x64_next = if amount_remaining_subtract_fee.gt(amount_in.as_ref().unwrap()) {
                sqrt_price_x64_target
            } else {
                SqrtPriceMath::get_next_sqrt_price_x64_from_input(sqrt_price_x64_current, liquidity, &amount_remaining_subtract_fee, zero_for_one)?
            };
            sqrt_price_x64_next
        } else {
            amount_out = Some(if zero_for_one {
                LiquidityMath::get_token_amount_b_from_liquidity(sqrt_price_x64_target, sqrt_price_x64_current, liquidity, false)?
            } else {
                LiquidityMath::get_token_amount_a_from_liquidity(sqrt_price_x64_current, sqrt_price_x64_target, liquidity, false)?
            });
            let sqrt_price_x64_next = if amount_remaining_neg.gt(amount_out.as_ref().unwrap()) {
                sqrt_price_x64_target
            } else {
                SqrtPriceMath::get_next_sqrt_price_x64_from_output(sqrt_price_x64_current, liquidity, &amount_remaining_neg, zero_for_one)?
            };
            sqrt_price_x64_next
        };
        let reach_target_price = sqrt_price_x64_target.eq(&sqrt_price_x64_next);
        if zero_for_one {
            if !(reach_target_price && base_input) {
                amount_in = Some(LiquidityMath::get_token_amount_a_from_liquidity(sqrt_price_x64_next, sqrt_price_x64_current, liquidity, true)?);
            }
            if !(reach_target_price && !base_input) {
                amount_out = Some(LiquidityMath::get_token_amount_b_from_liquidity(sqrt_price_x64_next, sqrt_price_x64_current, liquidity, false)?);
            }
        } else {
            amount_in = Some(if reach_target_price && base_input {
                amount_in.as_ref().unwrap().clone()
            } else {
                LiquidityMath::get_token_amount_b_from_liquidity(sqrt_price_x64_current, sqrt_price_x64_next, liquidity, true)?
            });
            amount_out = Some(if reach_target_price && !base_input {
                amount_out.as_ref().unwrap().clone()
            } else {
                LiquidityMath::get_token_amount_a_from_liquidity(sqrt_price_x64_current, sqrt_price_x64_next, liquidity, false)?
            });
        }
        if !base_input && amount_out.as_ref().unwrap().gt(&amount_remaining_neg) {
            amount_out = Some(amount_remaining_neg);
        }
        let fee_amount = if base_input && !sqrt_price_x64_next.eq(&sqrt_price_x64_target) {
            amount_remaining.sub(amount_in.as_ref().unwrap()).complete()
        } else {
            let a = amount_in.as_ref().unwrap();
            let b = rug::Integer::from_f32(fee_rate as f32).unwrap();
            let den = rug::Integer::from_f32((FEE_RATE_DENOMINATOR - fee_rate) as f32).unwrap();
            MathUtil::mul_div_ceil(a, &b, &den)?
        };
        Ok(SwapStepComputeResult {
            sqrt_price_x64: sqrt_price_x64_next,
            amount_in: amount_in.unwrap(),
            amount_out: amount_out.unwrap(),
            fee_amount,
        })
    }
}


fn to_twos(n: &Integer, bit_width: u32) -> Integer {
    // Если число отрицательное, переводим его в представление с двумя дополнениями.
    if n < &Integer::from(0) {
        n + (Integer::from(1) << bit_width)
    } else {
        n.clone()
    }
}

fn from_twos(n: &Integer, bit_width: u32) -> Integer {
    let threshold = Integer::from(1) << (bit_width - 1);
    if n >= &threshold {
        n - (Integer::from(1) << bit_width)
    } else {
        n.clone()
    }
}

fn signed_left_shift(n: &Integer, shift_by: u32, bit_width: u32) -> Integer {
    let mut twos = to_twos(n, bit_width);
    twos *= Integer::from(1) << shift_by;
    let mask = (Integer::from(1) << (bit_width + 1)) - 1;
    twos &= mask;
    from_twos(&twos, bit_width)
}

fn signed_right_shift(n: &Integer, shift_by: u32, bit_width: u32) -> Integer {
    let twos = to_twos(n, bit_width);
    let shifted = twos >> shift_by;
    from_twos(&shifted, bit_width)
}

fn mul_right_shift(val: &Integer, mul_by: &Integer) -> Integer {
    let product = (val * mul_by).complete();
    signed_right_shift(&product, 64, 256)
}

impl SqrtPriceMath {
    pub fn get_next_sqrt_price_from_token_amount_a_rounding_up(sqrt_price_x64: u128, liquidity: u128, amount: &rug::Integer, add: bool) -> Result<rug::Integer, String> {
        let sqrt_price_x64 = rug::Integer::from_str(&sqrt_price_x64.to_string()).unwrap();
        if amount.is_zero() {
            return Ok(sqrt_price_x64);
        }
        let liquidity_left_shift = rug::Integer::from_str(&liquidity.shl(U64_RESOLUTION).to_string()).unwrap();
        if add {
            let denominator = (&liquidity_left_shift).add(amount.mul(&sqrt_price_x64)).complete();
            let numerator1 = liquidity_left_shift;
            if denominator.ge(&numerator1) {
                return MathUtil::mul_div_ceil(&numerator1, &sqrt_price_x64, &denominator);
            }
            Ok(MathUtil::mul_div_rounding_up(&numerator1, &rug::Integer::ONE, &(&numerator1).div(&sqrt_price_x64).complete().add(amount)))
        } else {
            let amount_mul_sqrt_price = amount.mul(&sqrt_price_x64).complete();
            if !liquidity_left_shift.gt(&amount_mul_sqrt_price) {
                return Err("getNextSqrtPriceFromTokenAmountARoundingUp,liquidityLeftShift must gt amountMulSqrtPrice".to_string());
            }
            let denominator = (&liquidity_left_shift).sub(&amount_mul_sqrt_price).complete();
            MathUtil::mul_div_ceil(&liquidity_left_shift, &sqrt_price_x64, &denominator)
        }
    }
    pub fn get_next_sqrt_price_from_token_amount_b_rounding_down(sqrt_price_x64: u128, liquidity: u128, amount: &rug::Integer, add: bool) -> Result<rug::Integer, String> {
        let sqrt_price_x64 = rug::Integer::from_str(&sqrt_price_x64.to_string()).unwrap();
        let liquidity = rug::Integer::from_str(&liquidity.to_string()).unwrap();
        let delta_y = amount.shl(U64_RESOLUTION as i32).complete();
        if add {
            Ok(sqrt_price_x64.add(delta_y.div(liquidity)))
        } else {
            let amount_div_liquidity = MathUtil::mul_div_rounding_up(&delta_y, &rug::Integer::ONE, &liquidity);
            if !sqrt_price_x64.gt(&amount_div_liquidity) {
                return Err("getNextSqrtPriceFromTokenAmountBRoundingDown sqrtPriceX64 must gt amountDivLiquidity".to_string());
            }
            Ok(sqrt_price_x64.sub(amount_div_liquidity))
        }
    }
    pub fn get_next_sqrt_price_x64_from_input(sqrt_price_x64: u128, liquidity: u128, amount_in: &rug::Integer, zero_for_one: bool) -> Result<u128, String> {
        if !sqrt_price_x64.gt(&0) {
            return Err("sqrtPriceX64 must greater than 0".to_string());
        }
        if !liquidity.gt(&0) {
            return Err("liquidity must greater than 0".to_string());
        }
        let next = if zero_for_one {
            Self::get_next_sqrt_price_from_token_amount_a_rounding_up(sqrt_price_x64, liquidity, amount_in, true)
        } else {
            Self::get_next_sqrt_price_from_token_amount_b_rounding_down(sqrt_price_x64, liquidity, amount_in, true)
        };
        match next {
            Ok(x) => Ok(x.to_u128().unwrap()),
            Err(e) => Err(e)
        }
    }
    pub fn get_next_sqrt_price_x64_from_output(sqrt_price_x64: u128, liquidity: u128, amount_out: &rug::Integer, zero_for_one: bool) -> Result<u128, String> {
        if !sqrt_price_x64.gt(&0) {
            return Err("sqrtPriceX64 must greater than 0".to_string());
        }
        if !liquidity.gt(&0) {
            return Err("liquidity must greater than 0".to_string());
        }
        let next = if zero_for_one {
            Self::get_next_sqrt_price_from_token_amount_b_rounding_down(sqrt_price_x64, liquidity, amount_out, false)
        } else {
            Self::get_next_sqrt_price_from_token_amount_a_rounding_up(sqrt_price_x64, liquidity, amount_out, false)
        };
        match next {
            Ok(x) => Ok(x.to_u128().unwrap()),
            Err(e) => Err(e)
        }
    }
    fn get_tick_from_sqrt_price_x64(sqrt_price_x64: u128) -> Result<i32, String> {
        if sqrt_price_x64.gt(&MAX_SQRT_PRICE_X64) || sqrt_price_x64.lt(&MIN_SQRT_PRICE_X64) {
            return Err("Provided sqrtPrice is not within the supported sqrtPrice range.".to_string());
        }
        let msb = u128_bit_length(sqrt_price_x64) - 1;
        let adjusted_msb = rug::Integer::from_str(&(msb - 64).to_string()).unwrap();
        let log_2p_integer_x32 = signed_left_shift(&adjusted_msb, 32, 128);

        let mut bit = Integer::from_str_radix("8000000000000000", 16).unwrap();
        let mut precision = 0;
        let mut log_2p_fraction_x64 = rug::Integer::ZERO;
        let mut r = if msb >= 64 {
            sqrt_price_x64.shr(msb - 63)
        } else {
            sqrt_price_x64.shl(63 - msb)
        };

        while bit.gt(&rug::Integer::ZERO) && precision < BIT_PRECISION {
            r *= r;
            let r_more_than_two = r.shr(127);
            r = r.shr(63_u128 + r_more_than_two);
            log_2p_fraction_x64 += (&bit).mul(r_more_than_two);
            bit = bit.shr(1);
            precision += 1;
        }

        let log_2p_fraction_x32 = log_2p_fraction_x64.shr(32);

        let log_2p_x32: rug::Integer = log_2p_integer_x32.add(log_2p_fraction_x32);
        let log_bp_x64 = log_2p_x32.mul(rug::Integer::from_str(&LOG_B_2_X32).unwrap());

        let tick_low = signed_right_shift(&(&log_bp_x64).sub(rug::Integer::from_str(&LOG_B_P_ERR_MARGIN_LOWER_X64).unwrap()), 64, 128);
        let tick_high = signed_right_shift(&log_bp_x64.add(rug::Integer::from_str(&LOG_B_P_ERR_MARGIN_UPPER_X64).unwrap()), 64, 128);

        if tick_low == tick_high {
            Ok(tick_low.to_i32().unwrap())
        } else {
            let derived_tick_high_sqrt_price_x64 = SqrtPriceMath::get_sqrt_price_x64_from_tick(tick_high.to_i32().unwrap())?;
            if derived_tick_high_sqrt_price_x64.le(&sqrt_price_x64) {
                Ok(tick_high.to_i32().unwrap())
            } else {
                Ok(tick_low.to_i32().unwrap())
            }
        }
    }
    fn get_sqrt_price_x64_from_tick(tick: i32) -> Result<u128, String> {
        if tick < MIN_TICK || tick > MAX_TICK {
            return Err("tick must be in MIN_TICK and MAX_TICK".to_string());
        }
        let tick_abs = tick.abs();

        let mut ratio = if tick_abs & 0x1 != 0 {
            Integer::from_str("18445821805675395072").unwrap()
        } else {
            Integer::from_str("18446744073709551616").unwrap()
        };

        if tick_abs & 0x2 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("18444899583751176192").unwrap());
        }
        if tick_abs & 0x4 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("18443055278223355904").unwrap());
        }
        if tick_abs & 0x8 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("18439367220385607680").unwrap());
        }
        if tick_abs & 0x10 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("18431993317065453568").unwrap());
        }
        if tick_abs & 0x20 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("18417254355718170624").unwrap());
        }
        if tick_abs & 0x40 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("18387811781193609216").unwrap());
        }
        if tick_abs & 0x80 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("18329067761203558400").unwrap());
        }
        if tick_abs & 0x100 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("18212142134806163456").unwrap());
        }
        if tick_abs & 0x200 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("17980523815641700352").unwrap());
        }
        if tick_abs & 0x400 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("17526086738831433728").unwrap());
        }
        if tick_abs & 0x800 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("16651378430235570176").unwrap());
        }
        if tick_abs & 0x1000 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("15030750278694412288").unwrap());
        }
        if tick_abs & 0x2000 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("12247334978884435968").unwrap());
        }
        if tick_abs & 0x4000 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("8131365268886854656").unwrap());
        }
        if tick_abs & 0x8000 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("3584323654725218816").unwrap());
        }
        if tick_abs & 0x10000 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("696457651848324352").unwrap());
        }
        if tick_abs & 0x20000 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("26294789957507116").unwrap());
        }
        if tick_abs & 0x40000 != 0 {
            ratio = mul_right_shift(&ratio, &Integer::from_str("37481735321082").unwrap());
        }

        if tick > 0 {
            let max_uint128 = (Integer::from(1) << 128) - 1;
            ratio = max_uint128 / ratio;
        }
        Ok(ratio.to_u128().unwrap())
    }
}

impl MathUtil {
    pub fn mul_div_floor(a: &rug::Integer, b: &rug::Integer, denominator: &rug::Integer) -> Result<rug::Integer, String> {
        if denominator.is_zero() {
            return Err("division by 0".to_string());
        }
        Ok(a.mul(b).complete().div(denominator))
    }
    pub fn mul_div_ceil(a: &rug::Integer, b: &rug::Integer, denominator: &rug::Integer) -> Result<rug::Integer, String> {
        if denominator.is_zero() {
            return Err("division by 0".to_string());
        }
        let numerator = a.mul(b).complete().add(denominator.sub(rug::Integer::ONE).complete());
        Ok(numerator.div(denominator))
    }
    pub fn mul_div_rounding_up(a: &rug::Integer, b: &rug::Integer, denominator: &rug::Integer) -> rug::Integer {
        let numerator = a.mul(b).complete();
        let mut res = (&numerator / denominator).complete();
        if !numerator.modulo(denominator).eq(&rug::Integer::ZERO) {
            res += rug::Integer::ONE;
        }
        res
    }
}

impl LiquidityMath {
    pub fn get_token_amount_a_from_liquidity(sqrt_price_x64_a: u128, sqrt_price_x64_b: u128, liquidity: u128, round_up: bool) -> Result<rug::Integer, String> {
        let (a, b) = if sqrt_price_x64_a.gt(&sqrt_price_x64_b) {
            (sqrt_price_x64_b, sqrt_price_x64_a)
        } else {
            (sqrt_price_x64_a, sqrt_price_x64_b)
        };
        if !a.gt(&0) {
            return Err("(1) sqrtPriceX64A must greater than 0".to_string());
        }
        let numerator1 = rug::Integer::from_str(&(liquidity.clone() << U64_RESOLUTION).to_string()).unwrap();
        let numerator2 = rug::Integer::from_str(&b.sub(a).to_string()).unwrap();

        if round_up {
            let md3 = rug::Integer::from_str(&b.to_string()).unwrap();
            let m1 = MathUtil::mul_div_ceil(&numerator1, &numerator2, &md3)?;
            let m3 = rug::Integer::from_str(&a.to_string()).unwrap();
            Ok(MathUtil::mul_div_rounding_up(&m1, rug::Integer::ONE, &m3))
        } else {
            let m3 = rug::Integer::from_str(&b.to_string()).unwrap();
            MathUtil::mul_div_floor(&numerator1, &numerator2, &m3)
        }
    }

    pub fn get_token_amount_b_from_liquidity(sqrt_price_x64_a: u128, sqrt_price_x64_b: u128, liquidity: u128, round_up: bool) -> Result<rug::Integer, String> {
        let (a, b) = if sqrt_price_x64_a.gt(&sqrt_price_x64_b) {
            (sqrt_price_x64_b, sqrt_price_x64_a)
        } else {
            (sqrt_price_x64_a, sqrt_price_x64_b)
        };
        if !a.gt(&rug::Integer::ZERO) {
            return Err("(2) sqrtPriceX64A must greater than 0".to_string());
        }
        let liquidity = rug::Integer::from_str(&liquidity.to_string()).unwrap();
        let q64 = rug::Integer::from_str(&Q64.to_string()).unwrap();
        let b_sub_a = rug::Integer::from_str(&b.sub(a).to_string()).unwrap();
        if round_up {
            MathUtil::mul_div_ceil(&liquidity, &b_sub_a, &q64)
        } else {
            MathUtil::mul_div_floor(&liquidity, &b_sub_a, &q64)
        }
    }
}

fn u128_bit_length(x: u128) -> u32 {
    if x == 0 { 0 } else { 128 - x.leading_zeros() }
}