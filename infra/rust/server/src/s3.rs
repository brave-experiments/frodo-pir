use bytes::{buf::Reader, Buf};
use std::io::{Error, ErrorKind};

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;
use aws_smithy_http::byte_stream::AggregatedBytes;

const DEFAULT_REGION: &str = "us-west-2";

pub async fn init_client() -> Client {
  let region_provider =
    RegionProviderChain::default_provider().or_else(DEFAULT_REGION);
  let config = aws_config::from_env().region(region_provider).load().await;
  Client::new(&config)
}

pub async fn download_object(
  client: &Client,
  bucket_name: &str,
  key: &str,
) -> Result<Reader<AggregatedBytes>, Error> {
  let resp = match client
    .get_object()
    .bucket(bucket_name)
    .key(key)
    .send()
    .await
  {
    Ok(r) => r,
    Err(e) => {
      return Err(Error::new(
        ErrorKind::Other,
        format!("Err fetching object from S3: {}", e),
      ))
    }
  };

  let data = match resp.body.collect().await {
    Ok(d) => d,
    Err(e) => {
      return Err(Error::new(
        ErrorKind::Other,
        format!("Err reading body from S3: {}", e),
      ))
    }
  };

  Ok(data.reader())
}
