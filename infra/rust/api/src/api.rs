//! The `api` module provides server and client functions for running
//! the leaked credential checker.

pub use frodo_pir::api::{BaseParams, QueryParams as PIRQueryParams};
use frodo_pir::api::{
  CommonParams, Query as PIRQuery, Response as PIRResponse, Shard as Bucket,
};
use frodo_pir::errors::ResultBoxedError;

use std::fs;

use crate::errors::C3Error;
use sha2::{Digest, Sha256};

use serde::{Deserialize, Serialize};

use p256::NistP256;
use rand_core::OsRng;
use voprf::{BlindedElement, EvaluationElement, OprfClient, OprfServer};

/// The `ClientBucketParams` struct produces a set of parameters that
/// clients can use for preprocessing and launching queries. These
/// parameters are generated from the public `BaseParams` that are
/// generated for each server bucket.
#[derive(Serialize, Deserialize)]
pub struct ClientBucketParams {
  base: BaseParams,
  common: CommonParams,
}
impl From<BaseParams> for ClientBucketParams {
  fn from(bp: BaseParams) -> Self {
    let common = CommonParams::from(&bp);
    Self { base: bp, common }
  }
}

// Tracks the internal state that the client uses to process the server response
#[derive(Clone, Serialize, Deserialize)]
pub struct ClientState {
  pir_state: Vec<PIRQueryParams>,
  oprf_input: Vec<u8>,
  oprf_state: OprfClient<NistP256>,
}

// Give named types explicitly for readability
type PIRMessage = Vec<u8>;
type OPRFMessage = Vec<u8>;

// The message containing serialized data that a client sends to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMessage {
  pir_queries: Vec<PIRMessage>,
  oprf_query: OPRFMessage,
}

// The server response consists of the response to the `ClientMessage` that is sent
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerResponse {
  pir_resp: Vec<PIRMessage>,
  oprf_resp: OPRFMessage,
}

/// The `load_from_file` function loads the server bucket into memory,
/// from a text file containing hashed leaked credentials. It assumes
/// that each line contains a single hashed credential.
pub fn load_from_file(
  bucket_path: &str,
  lwe_dim: usize,
  element_bit_len: usize,
  db_matrix_entry_bit_len: usize,
) -> ResultBoxedError<Bucket> {
  load_bucket_from_hashes(
    &load_hashes_from_file(bucket_path)?,
    lwe_dim,
    element_bit_len,
    db_matrix_entry_bit_len,
  )
}

fn load_hashes_from_file(path: &str) -> Result<Vec<String>, C3Error> {
  let contents = fs::read_to_string(path)
    .map_err(|e| C3Error::LoadBucketError(path.into(), e.to_string()))?;
  Ok(contents.lines().map(String::from).collect())
}

pub fn load_bucket_from_hashes(
  hashes: &[String],
  lwe_dim: usize,
  element_bit_len: usize,
  db_matrix_entry_bit_len: usize,
) -> ResultBoxedError<Bucket> {
  Bucket::from_base64_strings(
    hashes,
    lwe_dim,
    hashes.len(),
    element_bit_len,
    db_matrix_entry_bit_len,
  )
}

/// The `client_get_bucket_id` function returns the bucket index that
/// should be queried, based on the queried username and the
/// `bucket_prefix_len` which corresponds to the number of hex
/// characters that are used.
pub fn client_get_bucket_id(
  username: &str,
  bucket_prefix_len: usize,
  total_buckets: u32,
) -> usize {
  crate::utils::get_mod_prefix(
    &Sha256::digest(&username),
    bucket_prefix_len,
    total_buckets,
  ) as usize
}

/// The `client_preproc_n_queries` function preprocesses `n` client
/// queries, this function can be run during the offline phase. The
/// results can be stored and used in the future. Each set of parameters
/// that is produced must only be used once.
pub fn client_preproc_n_queries(
  cbp: &ClientBucketParams,
  n: usize,
) -> ResultBoxedError<Vec<PIRQueryParams>> {
  (0..n)
    .into_iter()
    .map(|_| PIRQueryParams::new(&cbp.common, &cbp.base))
    .collect()
}

/// The `client_prepare_pir_query` function prepares their PIR query
/// index `idx` of the server bucket.
fn client_prepare_pir_query(
  qp: &mut PIRQueryParams,
  idx: usize,
) -> Result<Vec<u8>, C3Error> {
  let q = qp.prepare_query(idx).map_err(C3Error::PIRError)?;
  let se = bincode::serialize(&q).map_err(C3Error::SerdeError)?;
  Ok(se)
}

/// The `client_prepare_queries` function takes as input multiple sets
/// of params and db indices for the same credential (i.e. because the
/// client has to check multiple DB entries), and data to be queried to
/// an OPRF, and outputs a client message and corresponding state.
pub fn client_prepare_queries(
  qps: &[PIRQueryParams],
  db_indices: &[usize],
  oprf_input: &[u8],
) -> Result<(ClientMessage, ClientState), C3Error> {
  if qps.len() != db_indices.len() {
    return Err(C3Error::ClientQueryError(qps.len(), db_indices.len()));
  }
  let mut rng = OsRng;
  let mut pir_queries = Vec::with_capacity(db_indices.len());
  let mut updated_params = Vec::with_capacity(qps.len());
  for (i, qp) in qps.iter().enumerate() {
    let idx = db_indices[i];
    let mut params = qp.clone();
    let pir_query = client_prepare_pir_query(&mut params, idx)?;
    pir_queries.push(pir_query);
    updated_params.push(params)
  }
  let oprf_result = OprfClient::<NistP256>::blind(oprf_input, &mut rng)
    .map_err(C3Error::OprfError)?;
  let serialized_oprf_message =
    bincode::serialize(&oprf_result.message).map_err(C3Error::SerdeError)?;
  Ok((
    ClientMessage {
      pir_queries,
      oprf_query: serialized_oprf_message,
    },
    ClientState {
      pir_state: updated_params,
      oprf_input: oprf_input.to_vec(),
      oprf_state: oprf_result.state,
    },
  ))
}

/// The `server_calculate_response` function takes the queried bucket
/// and a serialized client query as input, and outputs a serialized
/// response.
pub fn server_calculate_response(
  bucket: &Bucket,
  cm: &ClientMessage,
  key_oprf: &[u8],
) -> Result<ServerResponse, C3Error> {
  let oprf_query: BlindedElement<NistP256> =
    bincode::deserialize(&cm.oprf_query).map_err(C3Error::SerdeError)?;
  // respond to OPRF query
  let oprf_server: OprfServer<NistP256> =
    OprfServer::new_with_key(key_oprf).map_err(C3Error::OprfError)?;
  let oprf_eval = oprf_server.evaluate(&oprf_query);
  let oprf_resp =
    bincode::serialize(&oprf_eval).map_err(C3Error::SerdeError)?;

  // respond to PIR queries
  let mut pir_resp = Vec::with_capacity(cm.pir_queries.len());
  for q in &cm.pir_queries {
    let pir_query: PIRQuery =
      bincode::deserialize(q).map_err(C3Error::SerdeError)?;
    let resp = bucket.respond(&pir_query).map_err(C3Error::PIRError)?;
    pir_resp.push(resp);
  }

  Ok(ServerResponse {
    pir_resp,
    oprf_resp,
  })
}

/// The `client_process_output` function receives the server response
/// and returns `Ok()` if one of the PIR responses matches the unblinded
/// OPRF response
pub fn client_process_output(
  sr: &ServerResponse,
  cs: &ClientState,
) -> Result<bool, C3Error> {
  // finalize oprf output
  let oprf_resp: EvaluationElement<NistP256> =
    bincode::deserialize(&sr.oprf_resp).map_err(C3Error::SerdeError)?;
  let oprf_final = cs
    .oprf_state
    .finalize(&cs.oprf_input, &oprf_resp)
    .map_err(C3Error::OprfError)?;

  // finalize PIR output
  let mut pir_outputs = Vec::with_capacity(sr.pir_resp.len());
  for (i, resp) in sr.pir_resp.iter().enumerate() {
    let pir_resp: PIRResponse =
      bincode::deserialize(resp).map_err(C3Error::SerdeError)?;
    let pir_final = pir_resp.parse_output_as_bytes(&cs.pir_state[i]);
    pir_outputs.push(pir_final);
  }

  // process OPRF response
  Ok(pir_outputs.iter().any(|o| o == oprf_final.as_slice()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::keyword::{KeywordIndexMapping, LocalHashPrefixTable};
  use std::path::PathBuf;

  const TEST_CREDENTIALS: [&str; 16] = [
    "poc-test@mail.com:poc-pwd",
    "some_email@email.com:some_password",
    "some_email@email.com:some_password1",
    "some_email@email.com:some_password2",
    "some_email1@email.com:some_password",
    "some_email1@email.com:some_password1",
    "some_email1@email.com:some_password2",
    "some_email2@email.com:some_password",
    "some_email2@email.com:some_password1",
    "some_email2@email.com:some_password2",
    "some_email3@email.com:some_password",
    "some_email3@email.com:some_password1",
    "some_email3@email.com:some_password2",
    "some_email4@email.com:some_password",
    "some_email4@email.com:some_password1",
    "some_email4@email.com:some_password2",
  ];

  fn e2e(bucket_path: &str, oprf_key_path: &str) {
    // server db
    let hashed =
      load_hashes_from_file(&format!("{}.bucket", bucket_path)).unwrap();
    // load the bucket and derive server parameters
    let bucket = load_bucket_from_hashes(&hashed, 1572, 256, 10).unwrap();
    // read oprf key
    let b64_oprf_key = std::fs::read_to_string(oprf_key_path).unwrap();
    let oprf_key = base64::decode(b64_oprf_key).unwrap();

    // Derive client parameters for the bucket
    let cbp = ClientBucketParams::from(bucket.get_base_params().clone());

    // client query preprocessing
    let query_params = client_preproc_n_queries(&cbp, hashed.len()).unwrap();
    for (idx, &c) in TEST_CREDENTIALS.iter().enumerate() {
      // prepare client query for row idx
      let cred = String::from(c).replace(':', "");
      let (cm, cs) = client_prepare_queries(
        &query_params[idx..idx + 1],
        &[idx],
        cred.as_bytes(),
      )
      .unwrap();

      // server response
      let sresp = server_calculate_response(&bucket, &cm, &oprf_key).unwrap();

      // parse response
      let output = client_process_output(&sresp, &cs);
      if let Ok(m) = output {
        assert!(m, "Failed to assert for {}", idx);
      } else {
        panic!("Error occurred for index {}: {:?}", idx, output.err());
      }
    }
  }

  fn e2e_by_local_keyword_mapping(bucket_path: &str, oprf_key_path: &str) {
    // load the bucket and derive server parameters
    let bucket =
      load_from_file(&format!("{}.bucket", bucket_path), 1572, 256, 10)
        .unwrap();
    // derive local keyword mapping
    let lkm =
      LocalHashPrefixTable::new_from_file(&format!("{}.lhp", bucket_path), 16);
    // read oprf key
    let b64_oprf_key = std::fs::read_to_string(oprf_key_path).unwrap();
    let oprf_key = base64::decode(b64_oprf_key).unwrap();

    // Derive client parameters for the bucket
    let cbp = ClientBucketParams::from(bucket.get_base_params().clone());
    // The following credentials have been added to the sample data manually
    let test_cred = String::from(TEST_CREDENTIALS[0]).replace(':', "");

    // preprocess client query
    let qps = client_preproc_n_queries(&cbp, 1).unwrap();
    // get index to query from keyword
    let indices = lkm.get_indices(test_cred.as_bytes());
    // The length could be more than one in theory, but the sample size is so small that the chance is tiny
    assert_eq!(indices.len(), 1);

    // complete PIR response
    let (cm, cs) = client_prepare_queries(
      &qps[..indices.len()],
      &indices,
      test_cred.as_bytes(),
    )
    .unwrap();
    let sresp = server_calculate_response(&bucket, &cm, &oprf_key).unwrap();
    let matched = client_process_output(&sresp, &cs).unwrap();
    assert!(matched);
  }

  #[test]
  fn end_to_end() {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bucket_path = d.join("../../test_data/buckets/0");
    let oprf_key_path = d.join("../../test_data/oprf_key");
    e2e(
      bucket_path.to_str().unwrap(),
      oprf_key_path.to_str().unwrap(),
    );
  }

  #[test]
  fn end_to_end_with_keyword() {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bucket_path = d.join("../../test_data/buckets/0");
    let oprf_key_path = d.join("../../test_data/oprf_key");
    e2e_by_local_keyword_mapping(
      bucket_path.to_str().unwrap(),
      oprf_key_path.to_str().unwrap(),
    );
  }
}
