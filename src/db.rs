use std::fs;
use std::io::BufReader;

use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::errors::ResultBoxedError;
use crate::utils::format::*;
use crate::utils::matrices::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Database {
  entries: Vec<Vec<u32>>,
  m: usize,
  ele_size: usize,
  plaintext_bits: usize,
}
impl Database {
  pub fn new(
    elements: &[String],
    m: usize,
    ele_size: usize,
    plaintext_bits: usize,
  ) -> ResultBoxedError<Self> {
    Ok(Self {
      entries: swap_matrix_fmt(&construct_rows(
        elements,
        m,
        ele_size,
        plaintext_bits,
      )?),
      m,
      ele_size,
      plaintext_bits,
    })
  }

  pub fn from_file(
    db_file: &str,
    m: usize,
    ele_size: usize,
    plaintext_bits: usize,
  ) -> ResultBoxedError<Self> {
    let file_contents: String = fs::read_to_string(db_file)?.parse()?;
    let elements: Vec<String> = serde_json::from_str(&file_contents)?;
    Self::new(&elements, m, ele_size, plaintext_bits)
  }

  pub fn switch_fmt(&mut self) {
    self.entries = swap_matrix_fmt(&self.entries);
  }

  pub fn vec_mult(&self, row: &[u32], col_idx: usize) -> u32 {
    let mut acc = 0u32;
    for (i, entry) in row.iter().enumerate() {
      acc = acc.wrapping_add(entry.wrapping_mul(self.entries[col_idx][i]));
    }
    acc
  }

  pub fn write_to_file(&self, path: &str) -> ResultBoxedError<()> {
    let json = json!(self.entries);
    Ok(serde_json::to_writer(&fs::File::create(path)?, &json)?)
  }

  /// Returns the ith row of the DB matrix
  pub fn get_row(&self, i: usize) -> Vec<u32> {
    self.entries[i].clone()
  }

  /// Returns the ith DB entry as a base64-encoded string
  pub fn get_db_entry(&self, i: usize) -> String {
    base64_from_u32_slice(
      &swap_matrix_fmt(&self.entries)[i],
      self.plaintext_bits,
      self.ele_size,
    )
  }

  /// Returns the width of the DB matrix
  pub fn get_matrix_width(element_size: usize, plaintext_bits: usize) -> usize {
    let mut quo = element_size / plaintext_bits;
    if element_size % plaintext_bits != 0 {
      quo += 1;
    }
    quo
  }

  /// Returns the width of the DB matrix
  pub fn get_matrix_width_self(&self) -> usize {
    Database::get_matrix_width(self.get_ele_size(), self.get_plaintext_bits())
  }

  /// Get the matrix size
  pub fn get_matrix_height(&self) -> usize {
    self.m
  }

  /// Get the element size
  pub fn get_ele_size(&self) -> usize {
    self.ele_size
  }

  /// Get the plaintext bits
  pub fn get_plaintext_bits(&self) -> usize {
    self.plaintext_bits
  }
}

/// The `BaseParams` object allows loading and interacting with params that
/// are used by the client for constructing queries
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BaseParams {
  dim: usize,
  m: usize,
  lhs_seed: [u8; 32],
  rhs: Vec<Vec<u32>>,
  ele_size: usize,
  plaintext_bits: usize,
}
impl BaseParams {
  pub fn new(db: &Database, dim: usize) -> Self {
    let lhs_seed = generate_seed();
    Self {
      lhs_seed,
      rhs: Self::generate_params_rhs(db, lhs_seed, dim, db.m),
      dim,
      m: db.m,
      ele_size: db.ele_size,
      plaintext_bits: db.plaintext_bits,
    }
  }

  /// Load params from a JSON file
  pub fn load(params_path: &str) -> ResultBoxedError<Self> {
    let reader = BufReader::new(fs::File::open(params_path)?);
    Ok(serde_json::from_reader(reader)?)
  }

  /// Generates the RHS of the params using the database and the seed
  /// for the LHS
  pub fn generate_params_rhs(
    db: &Database,
    lhs_seed: [u8; 32],
    dim: usize,
    m: usize,
  ) -> Vec<Vec<u32>> {
    let lhs = swap_matrix_fmt(&get_lwe_matrix_from_seed(lhs_seed, dim, m));
    (0..Database::get_matrix_width(db.ele_size, db.plaintext_bits))
      .map(|i| {
        let mut col = Vec::with_capacity(m);
        for r in &lhs {
          col.push(db.vec_mult(r, i));
        }
        col
      })
      .collect()
  }

  /// Writes the params struct as JSON to file
  pub fn write_to_file(&self, path: &str) -> ResultBoxedError<()> {
    let json = json!({
      "lhs_seed": self.lhs_seed,
      "rhs": self.rhs,
    });
    Ok(serde_json::to_writer(&fs::File::create(path)?, &json)?)
  }

  /// Computes s*(A*DB) using the RHS of the public parameters
  pub fn mult_right(&self, s: &[u32]) -> ResultBoxedError<Vec<u32>> {
    let cols = &self.rhs;
    (0..cols.len())
      .map(|i| vec_mult_u32_u32(s, &cols[i]))
      .collect()
  }

  pub fn get_total_records(&self) -> usize {
    self.m
  }

  pub fn get_dim(&self) -> usize {
    self.dim
  }

  pub fn get_ele_size(&self) -> usize {
    self.ele_size
  }

  pub fn get_plaintext_bits(&self) -> usize {
    self.plaintext_bits
  }
}

/// `CommonParams` holds the derived uniform matrix that is used for
/// constructing server public parameters and the client query.
#[derive(Serialize, Deserialize)]
pub struct CommonParams(Vec<Vec<u32>>);
impl CommonParams {
  // Returns the internal matrix
  pub fn as_matrix(&self) -> &[Vec<u32>] {
    &self.0
  }

  /// Computes s*A + e using the seed used to generate the LHS matrix of
  /// the public parameters
  pub fn mult_left(&self, s: &[u32]) -> ResultBoxedError<Vec<u32>> {
    let cols = self.as_matrix();
    (0..cols.len())
      .map(|i| {
        let s_a = vec_mult_u32_u32(s, &cols[i])?;
        let e = random_ternary();
        Ok(s_a.wrapping_add(e))
      })
      .collect()
  }
}
impl From<&BaseParams> for CommonParams {
  fn from(params: &BaseParams) -> Self {
    Self(get_lwe_matrix_from_seed(
      params.lhs_seed,
      params.dim,
      params.m,
    ))
  }
}

fn construct_rows(
  elements: &[String],
  m: usize,
  ele_size: usize,
  plaintext_bits: usize,
) -> ResultBoxedError<Vec<Vec<u32>>> {
  let row_width = Database::get_matrix_width(ele_size, plaintext_bits);

  let result = (0..m).map(|i| -> ResultBoxedError<Vec<u32>> {
    let mut row = Vec::with_capacity(row_width);
    let data = &elements[i];
    let bytes = base64::decode(data)?;
    let bits = bytes_to_bits_le(&bytes);
    for i in 0..row_width {
      let end_bound = (i + 1) * plaintext_bits;
      if end_bound < bits.len() {
        row.push(bits_to_u32_le(&bits[i * plaintext_bits..end_bound])?);
      } else {
        row.push(bits_to_u32_le(&bits[i * plaintext_bits..])?);
      }
    }
    Ok(row)
  });

  result.collect()
}

fn generate_seed() -> [u8; 32] {
  let mut seed = [0u8; 32];
  OsRng.fill_bytes(&mut seed);
  seed
}
