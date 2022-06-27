//! Utility modules for working with matrices and LWE conventions in the
//! PIR scheme of lwe-pir.

/// Functionality specific to the LWE setup that is used
pub mod lwe {
  const MODULUS: u64 = u32::MAX as u64 + 1;

  /// This value indicates the indicator value which is used to reveal
  /// the DB row that is queried.
  pub fn get_rounding_factor(plaintext_bits: usize) -> u32 {
    (MODULUS / get_plaintext_size(plaintext_bits) as u64) as u32
  }

  /// This value indicates the bound which indicates whether a bit in the
  /// row queried to the server is set to 0 (below), or 1 (above).
  pub fn get_rounding_floor(plaintext_bits: usize) -> u32 {
    get_rounding_factor(plaintext_bits) / 2
  }

  /// Returns the modulus for the plaintext space
  pub fn get_plaintext_size(plaintext_bits: usize) -> u32 {
    2u32.pow(plaintext_bits as u32)
  }
}

/// Functionality for matrix and vector manipulation
pub mod matrices {
  use rand::rngs::StdRng;
  use rand_core::{OsRng, RngCore, SeedableRng};

  /// Takes a matrix in row (column) format, and returns it in column (row) format
  pub fn swap_matrix_fmt(matrix: &[Vec<u32>]) -> Vec<Vec<u32>> {
    let height = matrix.len();
    let width = matrix[0].len(); // assumes all entries are the same size
    let mut swapped_row = vec![Vec::with_capacity(height); width];
    for current_row in matrix {
      for i in 0..width {
        swapped_row[i].push(current_row[i]);
      }
    }
    swapped_row
  }

  /// Generates an LWE matrix from a seed
  pub fn get_lwe_matrix_from_seed(
    seed: [u8; 32],
    lwe_dim: usize,
    width: usize,
  ) -> Vec<Vec<u32>> {
    let mut lhs = Vec::with_capacity(width);
    let mut rng = get_seeded_rng(seed);
    for _ in 0..width {
      let mut v = Vec::with_capacity(lwe_dim);
      for _ in 0..lwe_dim {
        v.push(rng.next_u32());
      }
      lhs.push(v);
    }
    lhs
  }

  /// Multiplies a u32 vector with a u32 column vector
  pub fn vec_mult_u32_u32(row: &[u32], col: &[u32]) -> u32 {
    if row.len() != col.len() {
      panic!("row_len: {}, col_len: {}", row.len(), col.len());
    }
    let mut acc = 0u32;
    for i in 0..row.len() {
      acc = acc.wrapping_add(row[i].wrapping_mul(col[i]));
    }
    acc
  }

  /// Returns a seeded RNG for sampling values
  fn get_seeded_rng(s: [u8; 32]) -> StdRng {
    StdRng::from_seed(s)
  }

  /// Simulates a ternary error by sampling randomly, using rejection
  /// sampling, from {0,1,u32::MAX}
  pub fn random_ternary() -> u32 {
    let zero_bound = u32::MAX / 3;
    let mut val = OsRng.next_u32();
    while val > zero_bound * 3 {
      // reject until below max bound
      val = OsRng.next_u32();
    }
    let mut tern = 0;
    if val > zero_bound && val <= zero_bound * 2 {
      tern = 1;
    } else if val > zero_bound * 2 {
      tern = u32::MAX;
    }
    tern
  }

  /// Simulates a ternary error vector of width size by sampling randomly,
  /// using rejection sampling, from {0,1,u32::MAX}
  pub fn random_ternary_vector(width: usize) -> Vec<u32> {
    let mut row = Vec::new();
    for _ in 0..width {
      row.push(random_ternary());
    }
    row
  }
}

/// Functionality related to manipulation of data formats that are used
pub mod format {
  use std::convert::TryInto;

  fn u8_to_bits_le(byte: u8) -> Vec<bool> {
    let mut ret = Vec::new();
    for i in 0..8 {
      ret.push(2u8.pow(i as u32) & byte > 0);
    }
    ret
  }

  pub fn u32_to_bits_le(x: u32, bit_len: usize) -> Vec<bool> {
    let bytes = x.to_le_bytes();
    let mut bits = Vec::with_capacity(bytes.len());
    for byte in bytes {
      bits.extend(u8_to_bits_le(byte));
    }
    bits[..bit_len].to_vec()
  }

  pub fn bits_to_bytes_le(bits: &[bool]) -> Vec<u8> {
    let mut bytes = vec![0u8; (bits.len() + 7) / 8];
    for (i, &bit) in bits.iter().enumerate() {
      if bit {
        let idx = ((i as f64) / 8f64).floor() as usize;
        let exp = (i % 8) as u32;
        bytes[idx] += 2u8.pow(exp);
      }
    }
    bytes
  }

  pub fn bytes_to_bits_le(bytes: &[u8]) -> Vec<bool> {
    bytes
      .iter()
      .map(|b| u8_to_bits_le(*b))
      .collect::<Vec<Vec<bool>>>()
      .iter()
      .fold(Vec::new(), |mut acc, next| {
        acc.extend(next);
        acc
      })
  }

  pub fn bits_to_u32_le(bits: &[bool]) -> u32 {
    let mut bytes = bits_to_bytes_le(bits);
    let u32_len = std::mem::size_of::<u32>();
    let byte_len = bytes.len();
    if byte_len > u32_len {
      panic!("bytes are too long to parse as u16, length: {}", byte_len);
    }
    let padding = vec![0u8; u32_len - byte_len];
    bytes.extend(padding);

    u32::from_le_bytes(u32_sized_bytes_from_vec(bytes))
  }

  pub fn u32_sized_bytes_from_vec(bytes: Vec<u8>) -> [u8; 4] {
    bytes.try_into().unwrap_or_else(|v: Vec<u8>| {
      panic!(
        "Expected a Vec of length {} but it was {}",
        std::mem::size_of::<u32>(),
        v.len()
      )
    })
  }

  pub fn base64_from_u32_slice(
    v: &[u32],
    entry_bit_len: usize,
    total_bit_len: usize,
  ) -> String {
    let remainder = total_bit_len % entry_bit_len;
    let mut bits = Vec::with_capacity(entry_bit_len * v.len());
    for i in 0..v.len() {
      // We extract either the full amount of bits, or the remainder from
      // the last index
      if i != v.len() - 1 {
        bits.extend(u32_to_bits_le(v[i], entry_bit_len));
      } else {
        bits.extend(u32_to_bits_le(v[i], remainder));
      }
    }
    let bytes = bits_to_bytes_le(&bits);
    base64::encode(bytes)
  }
}
