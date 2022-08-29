mod configs;
mod frodopir;
mod s3;

use crate::frodopir::BucketMetadata;
use actix_cors::Cors;
use actix_web::{get, post, web, App, HttpServer};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct ServerState {
  buckets_metadata: HashMap<String, BucketMetadata>,
  oprf_key: Vec<u8>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  let confs = match configs::get_env_configs() {
    Ok(confs) => confs,
    Err(err) => panic!("{}", err),
  };

  println!("> Init instance with configs: {:?}", confs);

  let s3_client = s3::init_client().await; // no op if running locally

  let buckets_metadata: HashMap<String, BucketMetadata> =
    frodopir::prepare_pub_params(
      &s3_client,
      confs.bucket,
      confs.shards,
      confs.shard_dir,
      confs.release,
    )
    .await?;

  let server_state = ServerState {
    buckets_metadata,
    oprf_key: confs.oprf_key,
  };

  HttpServer::new(move || {
    let cors = Cors::permissive();

    App::new()
      .app_data(web::Data::new(server_state.clone()))
      .app_data(web::JsonConfig::default().limit(5_242_880))
      .app_data(web::PayloadConfig::new(5_242_880))
      .wrap(cors)
      .service(query)
      .service(params)
  })
  .bind("0.0.0.0:".to_string() + &confs.port)?
  .run()
  .await
}

#[get("/params/{shard}")]
async fn params(
  shard_id: web::Path<usize>,
  data: web::Data<ServerState>,
) -> String {
  if !(data.buckets_metadata.contains_key(&shard_id.to_string())) {
    return "Shard not found".to_string();
  };

  println!("> Return public params for shard {}", shard_id);

  frodopir::pub_params_encoded(
    *shard_id,
    &data.buckets_metadata[&shard_id.to_string()],
  )
}

#[post("/query")]
async fn query(body: String, data: web::Data<ServerState>) -> String {
  println!("> Query received");
  frodopir::query_result_encoded(body, &data.buckets_metadata, &data.oprf_key)
    .unwrap()
}
