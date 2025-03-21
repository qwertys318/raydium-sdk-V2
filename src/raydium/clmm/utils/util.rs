pub fn is_zero(bit_num: u32, data: &rug::Integer) -> bool {
    for i in 0..bit_num {
        if data.get_bit(i) {
            return false;
        }
    }
    true
}
pub fn least_significant_bit(bit_num: u32, data: &rug::Integer) -> Option<u32> {
    if is_zero(bit_num, data) {
        return None;
    }
    Some(trailing_zeros(bit_num, data))
}
pub fn most_significant_bit(bit_num: u32, data: &rug::Integer) -> Option<u32> {
    if is_zero(bit_num, data) {
        return None
    }
    Some(leading_zeros(bit_num, data))
}
pub fn trailing_zeros(bit_num: u32, data: &rug::Integer) -> u32 {
    let mut i: u32 = 0;
    for j in 0..bit_num {
        if !data.get_bit(j) {
            i += 1;
        } else {
            break;
        }
    }
    i
}
pub fn leading_zeros(bit_num: u32, data: &rug::Integer) -> u32 {
    let mut i: u32 = 0;
    // Iterate from the highest bit (bit_num - 1) down to 0.
    for j in (0..bit_num).rev() {
        // get_bit returns 0 or 1 for the specified bit position.
        if !data.get_bit(j) {
            i += 1;
        } else {
            break;
        }
    }
    i
}
