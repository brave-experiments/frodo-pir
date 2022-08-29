use crate::s3;

use aws_sdk_s3::Client;
use std::{collections::HashMap, io::BufRead, time::Instant};

use frodo_pir::api::Shard as Bucket;
use leaked_creds_api::{
  api::{load_bucket_from_hashes, server_calculate_response},
  keyword::LocalHashPrefixTable,
  rpc::{
    ClientOnlineRequest, ServerOfflineResponse, ServerOnlineResponse,
    ValidateRequest,
  },
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::io::Error;
use std::path::Path;

// TODO: consider refactor to json config file
const LWE_DIM: usize = 1572;
const ELEMENT_BIT_LEN: usize = 256;
const DB_MATRIX_ENTRY_BIT_LEN: usize = 10;
const PREFIX_BIT_LEN: u32 = 16;
const BUCKET_METADATA_DIR: &str = "bucket_metadata";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketMetadata {
  pub bucket: Bucket,
  pub local_hash_prefix_table: LocalHashPrefixTable,
}

pub async fn prepare_pub_params(
  s3_client: &Client,
  s3_bucket: String,
  shards: Vec<String>,
  shard_dir: String,
  release: bool,
) -> Result<HashMap<String, BucketMetadata>, Error> {
  let mut shards_map: HashMap<String, BucketMetadata> = HashMap::new();
  if !Path::new(BUCKET_METADATA_DIR).is_dir() {
    std::fs::create_dir(BUCKET_METADATA_DIR)?;
  }

  for shard in shards {
    println!("> Loading and preparing shard {}...", shard);
    let mut elements: Vec<String> = vec![];
    let bmd_path = format!("{}/{}", BUCKET_METADATA_DIR, shard);

    let shard_path = shard_dir.clone() + "/" + &shard + ".bucket";

    if let Ok(s) = std::fs::read_to_string(&bmd_path) {
      println!(">> reading shard params from file");
      let bmd: BucketMetadata = serde_json::from_str(&s)?;
      shards_map.insert(shard.to_string(), bmd);
    } else {
      match release {
        true => {
          // release without cached content -> download it from S3
          println!(">> downloading bucket metadata from S3");
          let content =
            s3::download_object(s3_client, &s3_bucket, &shard_path).await?;
          println!(">> OK");
          let lines = content.lines();
          for line in lines {
            elements.push(line.unwrap());
          }
        }
        _ => {
          println!(">> getting db content from {} (local build)", shard_path);
          // local envronment
          let content: String = match fs::read_to_string(shard_path.clone()) {
            Ok(c) => c.parse().unwrap(),
            Err(e) => {
              println!(
                ">> error opening shard file in {} - Does it exist?",
                shard_path
              );
              return {
                Err(std::io::Error::new(futures::io::ErrorKind::NotFound, e))
              };
            }
          };
          let lines = content.lines();
          for line in lines {
            elements.push(line.to_string());
          }
        }
      };

      println!(">> load_bucket_from_hashes");
      let mut start = Instant::now();
      let bucket = load_bucket_from_hashes(
        &elements,
        LWE_DIM,
        ELEMENT_BIT_LEN,
        DB_MATRIX_ENTRY_BIT_LEN,
      )
      .unwrap();
      let mut checkpoint = start.elapsed();
      println!(">> OK {:?}", checkpoint);

      println!(">> LocalHashPrefixTable::new()");
      start = Instant::now();
      let local_hash_prefix_table =
        LocalHashPrefixTable::new(&bucket, PREFIX_BIT_LEN);
      checkpoint = start.elapsed();
      println!(">> OK {:?}", checkpoint);

      println!(">> shards_map.insert()");
      let bmd = BucketMetadata {
        bucket,
        local_hash_prefix_table,
      };
      start = Instant::now();
      shards_map.insert(shard.to_string(), bmd.clone());
      checkpoint = start.elapsed();
      println!(">> OK {:?}", checkpoint);

      println!(">> write bucket metadata to file");
      let json = json!(bmd);
      serde_json::to_writer(&std::fs::File::create(bmd_path)?, &json)?;
    }

    println!("Done, for shard {}", shard);
  }
  Ok(shards_map)
}

pub fn pub_params_encoded(
  shard_id: usize,
  container: &BucketMetadata,
) -> String {
  let bp = container.bucket.get_base_params();
  let sor = ServerOfflineResponse::from_local_hpt(
    bp,
    &container.local_hash_prefix_table,
    shard_id,
  );

  serde_json::to_string(&sor).unwrap()
}

pub fn query_result_encoded(
  client_payload: String,
  containers: &HashMap<String, BucketMetadata>,
  oprf_key: &[u8],
) -> Result<String, String> {
  let cor: ClientOnlineRequest = match serde_json::from_str(&client_payload) {
    Ok(c) => c,
    Err(e) => return Err(format!("Error decoding client query {}", e)),
  };
  cor.validate().unwrap();
  let shard_id = cor.params.bucket_id;
  println!("> Respond to query for shard {}", shard_id);

  let cm = cor.params.message;
  let sresp = server_calculate_response(
    &containers[&shard_id.to_string()].bucket,
    &cm.unwrap(),
    oprf_key,
  )
  .unwrap();

  let sor = ServerOnlineResponse::new(sresp, cor.id);
  let sor_json = serde_json::to_string(&sor).unwrap();

  Ok(sor_json)
}
