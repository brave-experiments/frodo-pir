//! The leaked-creds-checker crate builds an API for performing
//! privacy-preserving leaked credential checking. The underlying
//! cryptographic mechanism is based on the FrodoPIR scheme, for
//! performing Private Information Retrieval.
//!
//! # Example usage
//!
//! ## Offline preprocessing
//!
//! In the offline preprocessing phase, the server loads the database
//! and generates public parameters. The public parameters are
//! downloaded by the client, who can preprocess a number of queries to
//! be launched.
//!
//! ```
//! # use leaked_creds_api::api::*;
//! # use leaked_creds_api::keyword::*;
//! # let path = "../../test_data/buckets/0";
//! # let bucket_path = format!("{}.bucket", path);
//! # let lhp_path = format!("{}.lhp", path);
//! let lwe_dim = 1572; // see FrodoPIR paper for more details on this choice
//! let hashed_credential_bit_len = 256; // length of hashes used for each bucket element
//! let db_matrix_entry_bit_len = 10; // see FrodoPIR paper for more details on this choice
//!
//! // server load bucket from file
//! let bucket = load_from_file(
//!   &bucket_path,
//!   lwe_dim,
//!   hashed_credential_bit_len,
//!   db_matrix_entry_bit_len
//! ).unwrap();
//!
//! // server generates and publishes the local keyword-index mapping
//! // for online queries
//! let prefix_bit_len = 16; // shorter prefixes lead to more more client queries
//! let lkm = LocalHashPrefixTable::new_from_file(&lhp_path, prefix_bit_len);
//!
//! // send bucket.get_base_params() and lkm to client ...
//!
//! // Derive client parameters for the bucket
//! let cbp = ClientBucketParams::from(bucket.get_base_params().clone());
//!
//! // preprocess n = 16 queries
//! let preprocessed_query_params = client_preproc_n_queries(&cbp, 16).unwrap();
//! ```
//!
//! ## Online query
//!
//! In the online phase, the client makes a PIR query against the bucket
//! corresponding to their username. The client will learn whether their
//! username-password credential pair is present in the bucket.
//!
//! ```
//! # use leaked_creds_api::api::*;
//! # use leaked_creds_api::keyword::*;
//! # use rand_core::{OsRng};
//! # use voprf::{OprfClient, OprfServer, Ristretto255, BlindedElement, EvaluationElement};
//! # use sha2::{Sha256, Digest};
//! # let path = "../../test_data/buckets/0";
//! # let bucket_path = format!("{}.bucket", path);
//! # let lhp_path = format!("{}.lhp", path);
//! # let lwe_dim = 1572; // see FrodoPIR paper for more details on this choice
//! # let hashed_credential_bit_len = 256; // length of hashes used for each bucket element
//! # let db_matrix_entry_bit_len = 10;
//! # let bucket = load_from_file(
//! #   &bucket_path,
//! #   lwe_dim,
//! #   hashed_credential_bit_len,
//! #   db_matrix_entry_bit_len
//! # ).unwrap();
//! # let prefix_bit_len = 16;
//! # let lkm = LocalHashPrefixTable::new_from_file(&lhp_path, prefix_bit_len);
//! # let cbp = ClientBucketParams::from(bucket.get_base_params().clone());
//! # let mut preprocessed_query_params = client_preproc_n_queries(&cbp, 16).unwrap();
//! # let oprf_key_path = "../../test_data/oprf_key";
//! # let b64_oprf_key = std::fs::read_to_string(oprf_key_path).unwrap();
//! # let oprf_key = base64::decode(b64_oprf_key).unwrap();
//! // Client learns indices in server DB that need to be queried
//! let username = "poc-test@mail.com";
//! let password = "poc-pwd";
//! let data_to_hash = format!("{}{}", username, password);
//! let indices = lkm.get_indices(data_to_hash.as_bytes());
//!
//! // check if credential appears in server DB
//! let hash_chk: String = base64::encode(Sha256::digest(&data_to_hash));
//! let mut credential_leaked = false;
//! // client retrieve preprocessed query parameters
//!
//! // prepare client query for row indices
//! let (cm, cs) = client_prepare_queries(&preprocessed_query_params[..indices.len()], &indices, data_to_hash.as_bytes()).unwrap();
//!
//! // send client query to server...
//!
//! // server response
//! let sresp = server_calculate_response(&bucket, &cm, &oprf_key).unwrap();
//!
//! // return server response to client...
//!   
//! // client parses response
//! let output = client_process_output(&sresp, &cs).unwrap();
//! assert_eq!(output, true);
//! ```

pub mod api;
pub mod errors;
pub mod keyword;
pub mod rpc;
mod utils;
