//! The `rpc` module dictates a message format for requesting and
//! receiving data between the client and the server. The RPC format
//! uses JSONRPC version 2.0.
//!
//! # Example usage
//!
//! ## Offline request/response
//!
//! ```
//! # use leaked_creds_api::api::*;
//! # use leaked_creds_api::keyword::*;
//! # use leaked_creds_api::rpc::*;
//! # use sha2::Sha256;
//! # let path = "../../test_data/buckets/0";
//! # let bucket_path = format!("{}.bucket", path);
//! # let lhp_path = format!("{}.lhp", path);
//! # let lwe_dim = 1572;
//! # let hashed_credential_bit_len = 256;
//! # let db_matrix_entry_bit_len = 10;
//! # let bucket = load_from_file(
//! #   &bucket_path,
//! #   lwe_dim,
//! #   hashed_credential_bit_len,
//! #   db_matrix_entry_bit_len
//! # ).unwrap();
//! # let prefix_bit_len = 16;
//! # let hex_prefix_len = 15;
//! # let num_buckets = 1;
//! # let local_keyword_map = LocalHashPrefixTable::new_from_file(&lhp_path, prefix_bit_len);
//! // client-side...
//!
//! let username = "poc-test@mail.com";
//! let bucket_id: usize = client_get_bucket_id(username, hex_prefix_len, num_buckets);
//! let cor = ClientOfflineRequest::new(vec![bucket_id]);
//!
//! // send cor to server...
//!
//! // server-side...
//!
//! // validate request
//! cor.validate().unwrap();
//! // process response
//! let bp = bucket.get_base_params();
//! let sor = ServerOfflineResponse::from_local_hpt(&bp, &local_keyword_map, cor.id);
//!
//! // send sor to client...
//!
//! // client-side...
//! sor.validate(cor.id).unwrap();
//! let result = sor.result.unwrap();
//! // retrieve base_params
//! let (_bp, _local_hpt) = result.deserialize_local_hpt().unwrap();
//! let cbp = ClientBucketParams::from(_bp);
//! let _query_params = client_preproc_n_queries(&cbp, 16);
//!
//! // store _query_params and _local_hpt for online phase
//! ```
//!
//! ## Online request/response
//!
//! ```
//! # use leaked_creds_api::api::*;
//! # use leaked_creds_api::keyword::*;
//! # use leaked_creds_api::rpc::*;
//! # use sha2::Sha256;
//! # let path = "../../test_data/buckets/0";
//! # let bucket_path = format!("{}.bucket", path);
//! # let lhp_path = format!("{}.lhp", path);
//! # let lwe_dim = 1572;
//! # let hashed_credential_bit_len = 256;
//! # let db_matrix_entry_bit_len = 10;
//! # let bucket = load_from_file(
//! #   &bucket_path,
//! #   lwe_dim,
//! #   hashed_credential_bit_len,
//! #   db_matrix_entry_bit_len
//! # ).unwrap();
//! # let prefix_bit_len = 16;
//! # let hex_prefix_len = 15;
//! # let num_buckets = 1;
//! # let local_keyword_map = LocalHashPrefixTable::new_from_file(&lhp_path, prefix_bit_len);
//! # let username = "poc-test@mail.com";
//! # let bucket_id: usize = client_get_bucket_id(username, hex_prefix_len, num_buckets);
//! # let cor = ClientOfflineRequest::new(vec![bucket_id]);
//! # cor.validate().unwrap();
//! # let bp = bucket.get_base_params();
//! # let sor = ServerOfflineResponse::from_local_hpt(&bp, &local_keyword_map, cor.id);
//! # sor.validate(cor.id).unwrap();
//! # let result = sor.result.unwrap();
//! # let (_bp, local_hpt) = result.deserialize_local_hpt().unwrap();
//! # let cbp = ClientBucketParams::from(_bp);
//! # let mut query_params = client_preproc_n_queries(&cbp, 16).unwrap();
//! # let oprf_key_path = "../../test_data/oprf_key";
//! # let b64_oprf_key = std::fs::read_to_string(oprf_key_path).unwrap();
//! # let oprf_key = base64::decode(b64_oprf_key).unwrap();
//! // client-side...
//!
//! // Client learns indices in server DB that need to be queried
//! let username = "poc-test@mail.com";
//! let password = "poc-pwd";
//! let data_to_hash = format!("{}{}", username, password);
//! let bucket_id: usize = client_get_bucket_id(username, hex_prefix_len, num_buckets);
//! let indices = local_hpt.get_indices(data_to_hash.as_bytes());
//!
//! // prepare client query for row idx
//! assert!(indices.len() == 1);
//! let (cm, cs) = client_prepare_queries(&query_params[..indices.len()], &indices, data_to_hash.as_bytes()).unwrap();
//! let cor = ClientOnlineRequest::new(cm, bucket_id);
//!
//! // send cor to server...
//!
//! // server-side...
//!
//! cor.validate().unwrap();
//! let cm = cor.params.message.unwrap();
//!
//! let sresp = server_calculate_response(
//!   &bucket,
//!   &cm,
//!   &oprf_key,
//! ).unwrap();
//! let sor = ServerOnlineResponse::new(sresp, cor.id);
//!
//! // respond with sor to client ...
//!
//! // client-side...
//!
//! sor.validate(cor.id).unwrap();
//! let results = sor.result.unwrap();
//!
//! // parse server response ...
//! let output = client_process_output(&results, &cs).unwrap();
//! assert!(output)
//! ```
use crate::api::{BaseParams, ClientMessage, ServerResponse};
use crate::keyword::LocalHashPrefixTable;
use errors::RPCError;
use serde::{Deserialize, Serialize};

pub trait ValidateRequest {
  fn validate(&self) -> Result<(), RPCError>;
}
pub trait ValidateResponse {
  fn validate(&self, req_id: usize) -> Result<(), RPCError>;
}

#[derive(Deserialize, Serialize)]
pub struct ClientOfflineRequest {
  pub jsonrpc: String,
  pub method: String,
  pub params: Vec<usize>, // bucket identifiers to be retrieved
  pub id: usize,
}
impl ClientOfflineRequest {
  pub fn new(bucket_ids: Vec<usize>) -> Self {
    Self {
      params: bucket_ids,
      ..Default::default()
    }
  }
}
impl Default for ClientOfflineRequest {
  fn default() -> ClientOfflineRequest {
    ClientOfflineRequest {
      jsonrpc: "2.0".into(),
      method: "get_public_params".into(),
      params: vec![],
      id: 1,
    }
  }
}
impl ValidateRequest for ClientOfflineRequest {
  fn validate(&self) -> Result<(), RPCError> {
    if self.jsonrpc != "2.0" {
      return Err(RPCError::VersionValidation(self.jsonrpc.clone()));
    } else if self.params.is_empty() {
      return Err(RPCError::RequestParamsValidation(
        "Params object is empty".into(),
      ));
    }
    Ok(())
  }
}

#[derive(Deserialize, Serialize)]
pub struct KeywordIndexMappingMessage {
  pub method: String,
  pub data: String,
}
impl KeywordIndexMappingMessage {
  fn from_local_hpt(hpt: &LocalHashPrefixTable) -> Result<Self, RPCError> {
    let json = serde_json::to_string(hpt).map_err(RPCError::SerdeJSON)?;
    let data = base64::encode(&json);
    Ok(Self {
      method: "local_hpt".into(),
      data,
    })
  }
}

#[derive(Deserialize, Serialize)]
pub struct OfflineResponseResult {
  pub base_params: String, // base64-encoded base parameters
  pub kimm: KeywordIndexMappingMessage,
}
impl OfflineResponseResult {
  fn new(
    bp: &BaseParams,
    kimm: KeywordIndexMappingMessage,
  ) -> Result<Self, RPCError> {
    let json = serde_json::to_string(&bp).map_err(RPCError::SerdeJSON)?;
    Ok(Self {
      base_params: base64::encode(json.as_bytes()),
      kimm,
    })
  }

  pub fn deserialize_local_hpt(
    &self,
  ) -> Result<(BaseParams, LocalHashPrefixTable), RPCError> {
    if self.kimm.method != "local_hpt" {
      return Err(RPCError::ResultValidation(format!(
        "Invalid keyword mapping method specified ({})",
        self.kimm.method
      )));
    }
    let sbp = base64::decode(&self.base_params).map_err(RPCError::Base64)?;
    let bp = serde_json::from_slice(&sbp).map_err(RPCError::SerdeJSON)?;
    let local_hpt_data =
      base64::decode(&self.kimm.data).map_err(RPCError::Base64)?;
    let local_hpt =
      serde_json::from_slice(&local_hpt_data).map_err(RPCError::SerdeJSON)?;
    Ok((bp, local_hpt))
  }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResponseError {
  pub code: i32,
  pub message: String,
}
impl ResponseError {
  pub fn request_validation_error(e: RPCError) -> ResponseError {
    ResponseError {
      code: -32600,
      message: e.to_string(),
    }
  }

  pub fn offline_internal() -> ResponseError {
    ResponseError {
      code: -32602,
      message: "Error retrieving public parameters".into(),
    }
  }

  pub fn online_internal() -> ResponseError {
    ResponseError {
      code: -32602,
      message: "Failed to respond to query".into(),
    }
  }

  pub fn online_client_input() -> ResponseError {
    ResponseError {
      code: -32602,
      message: "Failed to deserialize client query".into(),
    }
  }
}

#[derive(Deserialize, Serialize)]
pub struct ServerOfflineResponse {
  pub jsonrpc: String,
  pub result: Option<OfflineResponseResult>,
  pub error: Option<ResponseError>,
  pub id: usize,
}
impl ServerOfflineResponse {
  pub fn from_local_hpt(
    base_params: &BaseParams,
    hpt: &LocalHashPrefixTable,
    id: usize,
  ) -> Self {
    let jsonrpc = "2.0";
    let err_resp = Self {
      jsonrpc: jsonrpc.into(),
      id,
      error: Some(ResponseError::offline_internal()),
      result: None,
    };
    let kimm_res = KeywordIndexMappingMessage::from_local_hpt(hpt);
    if kimm_res.is_err() {
      return err_resp;
    }
    let prr_res = OfflineResponseResult::new(base_params, kimm_res.unwrap());
    if prr_res.is_err() {
      return err_resp;
    }
    Self {
      jsonrpc: jsonrpc.into(),
      id,
      result: Some(prr_res.unwrap()),
      error: None,
    }
  }

  pub fn error(resp_err: ResponseError, id: usize) -> Self {
    Self {
      jsonrpc: "2.0".into(),
      id,
      error: Some(resp_err),
      result: None,
    }
  }
}
impl ValidateResponse for ServerOfflineResponse {
  fn validate(&self, req_id: usize) -> Result<(), RPCError> {
    if self.jsonrpc != "2.0" {
      return Err(RPCError::VersionValidation(self.jsonrpc.clone()));
    } else if self.id != req_id {
      return Err(RPCError::IDValidation(req_id, self.id));
    } else if let Some(e) = &self.error {
      return Err(RPCError::Response(e.code, e.message.clone()));
    } else if self.result.is_none() {
      return Err(RPCError::ResultValidation("No result object".into()));
    }
    Ok(())
  }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ClientRequestParams {
  pub bucket_id: usize,
  pub message: Option<ClientMessage>,
}
impl Default for ClientRequestParams {
  fn default() -> ClientRequestParams {
    ClientRequestParams {
      bucket_id: usize::MAX,
      message: None,
    }
  }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ClientOnlineRequest {
  pub jsonrpc: String,
  pub method: String,
  pub params: ClientRequestParams, // client base64-encoded query
  pub id: usize,
}
impl ClientOnlineRequest {
  pub fn new(cms: ClientMessage, bucket_id: usize) -> Self {
    Self {
      params: ClientRequestParams {
        message: Some(cms),
        bucket_id,
      },
      ..Default::default()
    }
  }
}
impl Default for ClientOnlineRequest {
  fn default() -> ClientOnlineRequest {
    ClientOnlineRequest {
      jsonrpc: "2.0".into(),
      method: "client_query".into(),
      params: ClientRequestParams::default(),
      id: 1,
    }
  }
}
impl ValidateRequest for ClientOnlineRequest {
  fn validate(&self) -> Result<(), RPCError> {
    if self.jsonrpc != "2.0" {
      return Err(RPCError::VersionValidation(self.jsonrpc.clone()));
    } else if self.params.message.is_none() {
      return Err(RPCError::RequestParamsValidation(
        "No Client message enclosed".into(),
      ));
    } else if self.params.bucket_id == usize::MAX {
      return Err(RPCError::RequestParamsValidation(
        "Bucket ID has not been set".into(),
      ));
    }
    Ok(())
  }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerOnlineResponse {
  pub jsonrpc: String,
  pub result: Option<ServerResponse>,
  pub error: Option<ResponseError>,
  pub id: usize,
}
impl ServerOnlineResponse {
  pub fn new(resps: ServerResponse, id: usize) -> Self {
    Self {
      jsonrpc: "2.0".into(),
      id,
      result: Some(resps),
      error: None,
    }
  }

  pub fn error(id: usize) -> Self {
    Self {
      jsonrpc: "2.0".into(),
      id,
      result: None,
      error: Some(ResponseError::online_internal()),
    }
  }
}
impl ValidateResponse for ServerOnlineResponse {
  fn validate(&self, req_id: usize) -> Result<(), RPCError> {
    if self.jsonrpc != "2.0" {
      return Err(RPCError::VersionValidation(self.jsonrpc.clone()));
    } else if self.id != req_id {
      return Err(RPCError::IDValidation(req_id, self.id));
    } else if let Some(e) = &self.error {
      return Err(RPCError::Response(e.code, e.message.clone()));
    } else if self.result.is_none() {
      return Err(RPCError::ResultValidation("No result object".into()));
    }
    Ok(())
  }
}

mod errors {
  #[derive(Debug)]
  pub enum RPCError {
    SerdeJSON(serde_json::Error),
    SerdeBincode(bincode::Error),
    Base64(base64::DecodeError),
    VersionValidation(String),
    RequestParamsValidation(String),
    ResultValidation(String),
    IDValidation(usize, usize),
    Response(i32, String),
  }

  impl std::error::Error for RPCError {}

  impl std::fmt::Display for RPCError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
      match self {
        RPCError::SerdeJSON(e) => write!(
          f,
          "Error occurred during JSON serialization of data: {}.",
          e
        ),
        RPCError::SerdeBincode(e) => write!(
          f,
          "Error occurred during bincode serialization of data: {}.",
          e
        ),
        RPCError::Base64(e) => {
          write!(f, "Error occurred during base64 decoding of data: {}.", e)
        }
        RPCError::VersionValidation(s) => write!(
          f,
          "JSONRPC: request version ({}) is incorrect, should be '2.0'.",
          s
        ),
        RPCError::RequestParamsValidation(s) => {
          write!(f, "JSONRPC: bad request params error, {}", s)
        }
        RPCError::IDValidation(id1, id2) => write!(
          f,
          "JSONRPC: bad response id error, request_id = {}, response_id = {}",
          id1, id2
        ),
        RPCError::ResultValidation(s) => {
          write!(f, "JSONRPC: bad response result error, {}", s)
        }
        RPCError::Response(c, s) => write!(
          f,
          "JSONRPC: response returned error, code: {}, message: {}",
          c, s
        ),
      }
    }
  }
}
