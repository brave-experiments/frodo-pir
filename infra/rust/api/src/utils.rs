use hex::ToHex;
use std::convert::TryInto;

pub fn get_prefix(bytes: &[u8], prefix_bit_len: u32) -> u32 {
  let u32_len = std::mem::size_of::<u32>();
  let mut val = u32::from_le_bytes(bytes[..u32_len].try_into().unwrap());
  if prefix_bit_len < 32 {
    val %= 2u32.pow(prefix_bit_len)
  }
  val
}

pub fn get_mod_prefix(bytes: &[u8], hex_prefix_len: usize, bound: u32) -> u32 {
  let h = bytes.encode_hex::<String>();
  let val = u64::from_str_radix(&h[..hex_prefix_len], 16).unwrap();
  (val % bound as u64) as u32
}
