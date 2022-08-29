//! The `errors` module provides a set of errors for exposing to client-
//! and server-side implementations.

use std::{error::Error, fmt};

// C3Error (Compromised Credential Checker Error) encapsulates the possible error cases of the
// leaked credentials APIs
#[derive(Debug)]
pub enum C3Error {
  LoadBucketError(String, String),
  PIRError(Box<dyn Error>),
  ClientQueryError(usize, usize),
  SerdeError(bincode::Error),
  KeywordIndexMappingError,
  KeywordIndexNotFoundError,
  OprfError(voprf::Error),
}

impl std::error::Error for C3Error {}

impl fmt::Display for C3Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      C3Error::LoadBucketError(path, s) => write!(
        f,
        "Error occurred loading bucket file at path {}, error: {}.",
        path, s
      ),
      C3Error::KeywordIndexMappingError => {
        write!(f, "Error occurred while processing KeywordIndexMapping",)
      }
      C3Error::KeywordIndexNotFoundError => {
        write!(f, "Error occurred, no index found for keyword",)
      }
      C3Error::PIRError(e) => {
        write!(f, "Error occurred in underlying PIR scheme: {}.", e)
      }
      C3Error::ClientQueryError(query_len, params_len) => {
        write!(f, "Error occurred as client specified inconsistent numbers of query parameters for each individual query, number of queries requested: {}, number of params objects: {}.", query_len, params_len)
      }
      C3Error::SerdeError(e) => {
        write!(f, "Error occurred during serialization of data: {}.", e)
      }
      C3Error::OprfError(e) => {
        write!(f, "Error occurred during operation of OPRF: {}.", e)
      }
    }
  }
}
