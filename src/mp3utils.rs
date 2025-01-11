pub fn get_bits(buffer: &[u8], start_bit: usize, end_bit: usize) -> u32 {
    let mut start_byte = start_bit >> 3;
    #[allow(unused_mut)]
    let mut end_byte = end_bit >> 3;
    let start_bit = start_bit % 8;
    let end_bit = end_bit % 8;

    let mut result = ((buffer[start_byte] as u32) << (32 - (8 - start_bit))) >> (32 - (8 - start_bit));

    if start_byte != end_byte {
        while start_byte + 1 != end_byte {
            result <<= 8;
            result += buffer[start_byte + 1] as u32;
            start_byte += 1;
        }
        result <<= end_bit;
        result += (buffer[end_byte] as u32) >> (8 - end_bit);
    } else if end_bit != 8 {
        result >>= 8 - end_bit;
    }

    result
}

pub fn get_bits_inc(buffer: &[u8], offset: &mut usize, count: usize) -> u32 {
    let result = get_bits(buffer, *offset, *offset + count);
    *offset += count;
    result
}

pub fn char_to_int(buffer: &[u8]) -> u32 {
    let mut num = 0x00;
    for &byte in buffer.iter() {
        num = (num << 7) + byte as u32;
    }
    num
}