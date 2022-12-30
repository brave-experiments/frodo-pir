//! Utility modules for working with matrices and LWE conventions in the
//! PIR scheme of lwe-pir.

/// Functionality specific to the LWE setup that is used
pub mod lwe {
  const MODULUS: u64 = u32::MAX as u64 + 1;

  /// Returns a value indicating the indicator value which is used to reveal
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

  use crate::errors::ErrorUnexpectedInputSize;
  use crate::errors::ResultBoxedError;

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

  /// Takes a matrix and returns the [*][i] elements
  /// equivalent to `swap_matrix_fmt(xys)[i]`, but much faster
  pub fn get_matrix_second_at(matrix: &[Vec<u32>], secidx: usize) -> Vec<u32> {
    matrix.iter().map(|y| y[secidx]).collect()
  }

  /// Generates an LWE matrix from a seed
  pub fn get_lwe_matrix_from_seed(
    seed: [u8; 32],
    lwe_dim: usize,
    width: usize,
  ) -> Vec<Vec<u32>> {
    let mut rng = get_seeded_rng(seed);
    (0..width).map(|_|
      (0..lwe_dim).map(|_| rng.next_u32()).collect()
    ).collect()
  }

  /// Multiplies a u32 vector with a u32 column vector
  pub fn vec_mult_u32_u32(row: &[u32], col: &[u32]) -> ResultBoxedError<u32> {
    if row.len() != col.len() {
      //panic!("row_len: {}, col_len: {}", row.len(), col.len());

      return Err(Box::new(ErrorUnexpectedInputSize::new(format!(
        "row_len: {}, col_len:{},",
        row.len(),
        col.len(),
      ))));
    }
    Ok(row.iter()
      .zip(col.iter())
      .map(|(&x, &y)| x.wrapping_mul(y))
      .fold(0u32, |acc, i| acc.wrapping_add(i)))
  }

  /// Returns a seeded RNG for sampling values
  fn get_seeded_rng(s: [u8; 32]) -> StdRng {
    StdRng::from_seed(s)
  }

  // Values used to denote the size of intervals that are used for
  // sampling ternary values, and a max bound that dictates when
  // randomly sampled values should be rejected.
  const TERNARY_INTERVAL_SIZE: u32 = (u32::MAX - 2) / 3;
  // Note `TERNARY_REJECTION_SAMPLING_MAX ≠ u32::MAX`
  const TERNARY_REJECTION_SAMPLING_MAX: u32 = TERNARY_INTERVAL_SIZE * 3;

  /// Simulates a ternary error by sampling randomly, using rejection
  /// sampling, from {0,1,u32::MAX} which is equivalent to {0,1,-1} when
  /// performing modular reduction.
  pub fn random_ternary() -> u32 {
    // We need to do rejection sampling for sampling randomly from 3
    // possible values: we first divide the full interval by 3, noting
    // that rounding is performed to the next _lowest_ integer.
    let mut val = OsRng.next_u32();
    // If the value sampled sits in the interval:
    //                `interval*3 < val < U32::MAX`
    // then we need to reject it and resample until it firs below `interval*3`
    while val > TERNARY_REJECTION_SAMPLING_MAX {
      val = OsRng.next_u32();
    }
    // Now we return {0,1,-1} depending on whether the sampled value
    // sits in the first, second or third sampling interval
    if val > TERNARY_INTERVAL_SIZE && val <= TERNARY_INTERVAL_SIZE * 2 {
      1
    } else if val > TERNARY_INTERVAL_SIZE * 2 {
      u32::MAX
    } else {
      0
    }
  }

  /// Simulates a ternary error vector of width size by sampling randomly,
  /// using rejection sampling, from {0,1,u32::MAX}
  pub fn random_ternary_vector(width: usize) -> Vec<u32> {
    (0..width).map(|_| random_ternary()).collect()
  }
}

/// Functionality related to manipulation of data formats that are used
pub mod format {
  use crate::errors::ErrorUnexpectedInputSize;
  use std::convert::TryInto;

  fn u8_to_bits_le(byte: u8) -> [bool; 8] {
    [
      2u8.pow(0) & byte != 0,
      2u8.pow(1) & byte != 0,
      2u8.pow(2) & byte != 0,
      2u8.pow(3) & byte != 0,

      2u8.pow(4) & byte != 0,
      2u8.pow(5) & byte != 0,
      2u8.pow(6) & byte != 0,
      2u8.pow(7) & byte != 0,
    ]
  }

  fn bits_to_u8_le(xs: &[bool]) -> u8 {
    assert!(xs.len() <= 8);
    xs.iter()
      .enumerate()
      .filter(|(_, &bit)| bit)
      .map(|(i, _)| 2u8.pow(i as u32))
      .sum()
  }

  pub fn u32_to_bits_le(x: u32, bit_len: usize) -> Vec<bool> {
    x.to_le_bytes()
      .into_iter()
      .flat_map(u8_to_bits_le)
      .take(bit_len)
      .collect()
  }

  pub fn bits_to_bytes_le(bits: &[bool]) -> Vec<u8> {
    bits.chunks(8)
      .map(|bits8| bits_to_u8_le(bits8))
      .collect()
  }

  pub fn bytes_to_bits_le(bytes: &[u8]) -> Vec<bool> {
    bytes.iter()
      .copied()
      .flat_map(u8_to_bits_le)
      .collect()
  }

  pub fn bits_to_u32_le(
    bits: &[bool],
  ) -> Result<u32, ErrorUnexpectedInputSize> {
    bytes_to_u32_le(bits_to_bytes_le(bits))
  }

  pub fn bytes_to_u32_le(
    mut bytes: Vec<u8>,
  ) -> Result<u32, ErrorUnexpectedInputSize> {
    let u32_len = core::mem::size_of::<u32>();
    let byte_len = bytes.len();
    if byte_len > u32_len {
      return Err(ErrorUnexpectedInputSize::new(format!(
        "bytes are too long to parse as u16, length: {}",
        byte_len
      )));
    }
    let padding = vec![0u8; u32_len - byte_len];
    bytes.extend(padding);

    Ok(u32::from_le_bytes(u32_sized_bytes_from_vec(bytes)?))
  }

  pub fn u32_sized_bytes_from_vec(
    bytes: Vec<u8>,
  ) -> Result<[u8; 4], ErrorUnexpectedInputSize> {
    bytes.try_into()
      .map_err(|e| ErrorUnexpectedInputSize::new(format!(
        "Unexpected vector size: {:?}",
        e,
      )))
  }

  pub fn bytes_from_u32_slice(
    v: &[u32],
    entry_bit_len: usize,
    total_bit_len: usize,
  ) -> Vec<u8> {
    let remainder = total_bit_len % entry_bit_len;
    let bits: Vec<_> = v.iter().enumerate().flat_map(|(i, &vi)| {
      // We extract either the full amount of bits, or the remainder from
      // the last index
      u32_to_bits_le(vi, if i != v.len() - 1 {
        entry_bit_len
      } else {
        remainder
      })
    }).collect();
    bits_to_bytes_le(&bits)
  }

  pub fn base64_from_u32_slice(
    v: &[u32],
    entry_bit_len: usize,
    total_bit_len: usize,
  ) -> String {
    base64::encode(bytes_from_u32_slice(v, entry_bit_len, total_bit_len))
  }
}
