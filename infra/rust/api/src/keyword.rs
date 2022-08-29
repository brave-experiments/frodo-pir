//! The `keyword` module provides a set of traits and structs that
//! implement functionality for mapping client keywords to server DB
//! indices that can be queried using the underlying PIR scheme.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use frodo_pir::api::Shard as Bucket;

/// The `KeywordIndexMapping` trait produces an interface by which
/// clients can learn the correct index (or indices) in the server DB
/// that must be queried. The input to the interface is their keyword
/// (i.e. the credential that they want to check), and the output is a
/// set of indices that they should query.
///
/// Note that the KeywordIndexMapping should be a very small fraction of
/// the total DB size, and mut not reveal anything about the server DB.
pub trait KeywordIndexMapping {
  /// The `get_indices` function should map a client leaked credential
  /// to the DB indices that it is related to. If the response vector is
  /// empty, then no queries are required.
  fn get_indices(&self, keyword: &[u8]) -> Vec<usize>;
}

/// The `LocalHashPrefixTable` struct allows clients to perform
/// keyword-index mapping for PIR queries using a local storage table
/// containing prefixes of each of the credentials in the server DB. The
/// table is produced by the server, and should be downloaded by the
/// client. The client must query all indices that have hash prefixes
/// matching the hash prefix of their keyword.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LocalHashPrefixTable {
  prefixes: Vec<u32>,
  prefix_bit_len: u32,
}
impl LocalHashPrefixTable {
  /// Function called by the server to generate the hash prefix table
  pub fn new(bucket: &Bucket, prefix_bit_len: u32) -> Self {
    let prefixes = bucket
      .into_row_iter()
      .map(|row| {
        let bytes = base64::decode(row).unwrap();
        LocalHashPrefixTable::get_prefix(&bytes, prefix_bit_len)
      })
      .collect();
    Self {
      prefixes,
      prefix_bit_len,
    }
  }

  // Instantiates the table from file
  pub fn new_from_file(path: &str, prefix_bit_len: u32) -> Self {
    let contents: String =
      std::fs::read_to_string(path).unwrap().parse().unwrap();
    let prefixes_b64: Vec<String> =
      contents.lines().map(String::from).collect();
    let prefixes = prefixes_b64
      .iter()
      .map(|b64p| {
        let bytes = base64::decode(b64p).unwrap();
        LocalHashPrefixTable::get_prefix(&bytes, prefix_bit_len)
      })
      .collect();
    Self {
      prefixes,
      prefix_bit_len,
    }
  }

  fn get_prefix(bytes: &[u8], prefix_bit_len: u32) -> u32 {
    crate::utils::get_prefix(bytes, prefix_bit_len)
  }

  pub fn len(&self) -> usize {
    self.prefixes.len()
  }

  pub fn is_empty(&self) -> bool {
    self.prefixes.is_empty()
  }
}
impl KeywordIndexMapping for LocalHashPrefixTable {
  fn get_indices(&self, keyword: &[u8]) -> Vec<usize> {
    let hash = Sha256::digest(keyword);
    let mut indices = Vec::new();
    for (i, &p) in self.prefixes.iter().enumerate() {
      let pref =
        LocalHashPrefixTable::get_prefix(&hash[..4], self.prefix_bit_len);
      if p == pref {
        indices.push(i)
      }
    }
    indices
  }
}
