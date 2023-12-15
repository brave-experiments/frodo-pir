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
  elem_size: usize,
  plaintext_bits: usize,
}

impl Database {
  pub fn new(
    elements: &[String],
    m: usize,
    elem_size: usize,
    plaintext_bits: usize,
  ) -> ResultBoxedError<Self> {
    Ok(Self {
      entries: swap_matrix_fmt(&construct_rows(
        elements,
        m,
        elem_size,
        plaintext_bits,
      )?),
      m,
      elem_size,
      plaintext_bits,
    })
  }

  pub fn from_file(
    db_file: &str,
    m: usize,
    elem_size: usize,
    plaintext_bits: usize,
  ) -> ResultBoxedError<Self> {
    let file_contents: String = fs::read_to_string(db_file)?.parse()?;
    let elements: Vec<String> = serde_json::from_str(&file_contents)?;
    Self::new(&elements, m, elem_size, plaintext_bits)
  }

  pub fn switch_fmt(&mut self) {
    self.entries = swap_matrix_fmt(&self.entries);
  }

  pub fn vec_mult(&self, row: &[u32], col_idx: usize) -> u32 {
    vec_mult_u32_u32(row, &self.entries[col_idx]).unwrap()
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
      &get_matrix_second_at(&self.entries, i),
      self.plaintext_bits,
      self.elem_size,
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
    Database::get_matrix_width(self.get_elem_size(), self.get_plaintext_bits())
  }

  /// Get the matrix size
  pub fn get_matrix_height(&self) -> usize {
    self.m
  }

  /// Get the element size
  pub fn get_elem_size(&self) -> usize {
    self.elem_size
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
  dim: usize, // the lwe dimension

  m: usize,         // the size of the DB
  elem_size: usize, // the size (in bits) of each element of the DB. Corresponds to `w` in paper.
  plaintext_bits: usize,

  public_seed: [u8; 32],
  rhs: Vec<Vec<u32>>,
}

impl BaseParams {
  pub fn new(db: &Database, dim: usize) -> Self {
    let public_seed = generate_seed(); // generates the public seed
    Self {
      public_seed,
      rhs: Self::generate_params_rhs(db, public_seed, dim, db.m),
      dim,
      m: db.m,
      elem_size: db.elem_size,
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
    public_seed: [u8; 32],
    dim: usize,
    m: usize,
  ) -> Vec<Vec<u32>> {
    let lhs =
      swap_matrix_fmt(&generate_lwe_matrix_from_seed(public_seed, dim, m));
    (0..db.get_matrix_width_self())
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
      "public_seed": self.public_seed,
      "rhs": self.rhs,
    });
    Ok(serde_json::to_writer(&fs::File::create(path)?, &json)?)
  }

  /// Computes c = s*(A*DB) using the RHS of the public parameters
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

  pub fn get_elem_size(&self) -> usize {
    self.elem_size
  }

  pub fn get_plaintext_bits(&self) -> usize {
    self.plaintext_bits
  }
}

/// `CommonParams` holds the derived uniform matrix that is used for
/// constructing the server's public parameters and the client query.
#[derive(Serialize, Deserialize)]
pub struct CommonParams(Vec<Vec<u32>>);
impl CommonParams {
  // Returns the internal matrix
  pub fn as_matrix(&self) -> &[Vec<u32>] {
    &self.0
  }

  /// Computes b = s*A + e using the seed used to generate the matrix of
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
    Self(generate_lwe_matrix_from_seed(
      params.public_seed,
      params.dim,
      params.m,
    ))
  }
}

fn construct_rows(
  elements: &[String],
  m: usize,
  elem_size: usize,
  plaintext_bits: usize,
) -> ResultBoxedError<Vec<Vec<u32>>> {
  let row_width = Database::get_matrix_width(elem_size, plaintext_bits);

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
