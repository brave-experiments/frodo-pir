use std::{env, fmt};

pub const ENV_LOCAL: &str = "local";
pub const ENV_RELEASE: &str = "release";

#[derive(Debug, Clone)]
pub struct ServerConfig {
  pub port: String,
  pub shard_dir: String,
  pub shards: Vec<String>,
  pub bucket: String,
  pub oprf_key: Vec<u8>,
  pub release: bool,
}

#[derive(Debug, Clone)]
pub struct ServerConfigErr {
  reason: String,
}

impl fmt::Display for ServerConfigErr {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Server configs error: {}", self.reason)
  }
}

impl ServerConfigErr {
  fn throw(reason: &str) -> Self {
    Self {
      reason: reason.to_string(),
    }
  }
}

pub fn get_env_configs() -> Result<ServerConfig, ServerConfigErr> {
  let env = match env::var("ENV") {
    Ok(e) => e.to_string(),
    Err(_) => ENV_RELEASE.to_string(),
  };

  match env.as_str() {
    ENV_LOCAL => get_configs_local(),
    _ => get_configs_release(),
  }
}

pub fn get_configs_local() -> Result<ServerConfig, ServerConfigErr> {
  let (port, shard_dir, shards, oprf_key) = match get_common_configs() {
    Ok(results) => results,
    Err(e) => return Err(ServerConfigErr::throw(&e)),
  };

  Ok(ServerConfig {
    port,
    shard_dir,
    shards,
    bucket: "nan".to_string(),
    oprf_key,
    release: false,
  })
}

pub fn get_configs_release() -> Result<ServerConfig, ServerConfigErr> {
  let (port, shard_dir, shards, oprf_key) = match get_common_configs() {
    Ok(results) => results,
    Err(e) => return Err(ServerConfigErr::throw(&e)),
  };

  let bucket = match env::var("BUCKET") {
    Ok(v) => v,
    Err(_) => return Err(ServerConfigErr::throw("BUCKET should be provided")),
  };

  Ok(ServerConfig {
    port,
    shard_dir,
    shards,
    bucket,
    oprf_key,
    release: true,
  })
}

fn get_common_configs() -> Result<(String, String, Vec<String>, Vec<u8>), String>
{
  let port = match env::var("PORT") {
    Ok(v) => v,
    Err(_) => return Err("PORT should be provided".to_string()),
  };

  let shard_dir = match env::var("SHARD_DIR") {
    Ok(v) => v,
    Err(_) => {
      return Err("SHARD_DIR should be provided".to_string());
    }
  };

  let shards = match env::var("SHARDS_INTERVAL".to_string()) {
    Ok(v) => {
      let shard_bounds: Vec<String> = v
        .split('-')
        .map(|x| x.to_string().replace(",", ""))
        .collect();

      if shard_bounds.len() != 2 {
        return Err(
          "SHARDS_INTERVAL should be an interval, e.g. `3-10`".to_string(),
        );
      }
      let left_bound: usize = match shard_bounds[0].parse() {
        Ok(v) => v,
        Err(e) => {
          panic!(
            "Error parsing SHARDS_INTERVAL left bound {:?}: {}",
            shard_bounds, e
          )
        }
      };
      let right_bound: usize = match shard_bounds[1].parse() {
        Ok(v) => v,
        Err(e) => {
          panic!(
            "Error parsing SHARDS_INTERVAL right bound {:?}: {}",
            shard_bounds, e
          )
        }
      };

      let mut shards: Vec<String> = vec![];
      for i in left_bound..right_bound + 1 {
        shards.push(i.to_string());
      }
      shards
    }
    Err(_) => {
      return Err("SHARDS_INTERVAL should be provided".to_string());
    }
  };

  let oprf_key_res = match env::var("OPRF_KEY") {
    Ok(v) => base64::decode(v.replace(",", "")),
    Err(_) => {
      return Err("OPRF_KEY should be provided".to_string());
    }
  };

  let oprf_key: Vec<u8> = match oprf_key_res {
    Ok(v) => v,
    Err(e) => {
      return Err(format!(
        "Invalid OPRF_KEY format ({:?}): {}",
        env::var("OPRF_KEY"),
        e
      ));
    }
  };

  Ok((port, shard_dir, shards, oprf_key))
}
