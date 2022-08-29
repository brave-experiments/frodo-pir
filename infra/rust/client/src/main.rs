use leaked_creds_api::{
  api::{
    client_get_bucket_id, client_prepare_queries, client_preproc_n_queries,
    client_process_output, BaseParams, ClientBucketParams, PIRQueryParams,
  },
  keyword::{KeywordIndexMapping, LocalHashPrefixTable},
  rpc::{
    ClientOnlineRequest, ServerOfflineResponse, ServerOnlineResponse,
    ValidateResponse,
  },
};

use std::path::Path;
use std::str;

use clap::{App, Arg};

use serde::{Deserialize, Serialize};
use serde_json::json;

const HEX_PREFIX_LEN: usize = 16;
const BUCKETS_TOTAL_DEFAULT: usize = 16;
const BUCKETS_PER_INSTANCE_DEFAULT: usize = 4;
const REMOTE_INSTANCE_URLS_DEFAULT: [&str; 4] = [
  "ec2-54-184-23-71.us-west-2.compute.amazonaws.com:8080",
  "ec2-52-89-9-75.us-west-2.compute.amazonaws.com:8080",
  "ec2-54-201-141-198.us-west-2.compute.amazonaws.com:8080",
  "ec2-54-212-69-254.us-west-2.compute.amazonaws.com:8080",
];
const LOCAL_CACHE_DATA_DIR: &str = "bucket_metadata";

#[derive(Debug, Serialize, Deserialize)]
struct Config {
  db_content_path: String,
  buckets_path: String,
  oprf_key: String,
  buckets_per_instance: usize,
  instances: Vec<ConfigInstances>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigInstances {
  port: String,
  shards: String,
}

struct BucketInfo {
  params_url: String,
  query_url: String,
}
impl BucketInfo {
  fn new(
    id: usize,
    instance_urls: Vec<String>,
    buckets_per_instance: usize,
  ) -> Self {
    let mut base_url = String::from("http://");
    if id >= buckets_per_instance * instance_urls.len() {
      panic!("Bad index mapping selected");
    }
    for (i, instance) in instance_urls.iter().enumerate() {
      if id < buckets_per_instance * (i + 1) {
        base_url += instance;
        break;
      }
    }

    Self {
      params_url: base_url.to_string() + "/params",
      query_url: base_url.to_string() + "/query",
    }
  }
}

pub struct Credential {
  pub username: String,
  pub password: String,
  pub n_preprocess: usize,
  pub config_path: String,
  pub local_env: bool,
}

impl Credential {
  pub fn parse_from_cli_flags() -> Self {
    let matches = App::new("PIR example")
      .version("0.0.1")
      .author("Alex Davidson <coela@alxdavids.xyz>")
      .about("Flags for setting PIR parameters")
      .arg(
        Arg::with_name("username")
          .short("u")
          .long("username")
          .takes_value(true)
          .help("Username to query"),
      )
      .arg(
        Arg::with_name("password")
          .short("p")
          .long("password")
          .takes_value(true)
          .help("Password to query"),
      )
      .arg(
        Arg::with_name("n_preprocess")
          .short("n")
          .long("n_preprocess")
          .takes_value(true)
          .default_value("1")
          .help("Minimum number of queries to preprocess"),
      )
      .arg(
        Arg::with_name("config")
          .short("c")
          .long("config")
          .takes_value(true)
          .default_value("")
          .help("Configuration file path"),
      )
      .get_matches();

    let username = match matches.value_of("username") {
      Some(u) => String::from(u),
      None => panic!("--username not provided"),
    };

    let password = match matches.value_of("password") {
      Some(u) => String::from(u),
      None => panic!("--password not provided"),
    };

    let n_preprocess: usize =
      String::from(matches.value_of("n_preprocess").unwrap())
        .parse()
        .unwrap();

    let config_path = String::from(matches.value_of("config").unwrap())
      .parse()
      .unwrap();

    let mut local_env = false;
    if config_path != "" {
      local_env = true;
    }

    Self {
      username,
      password,
      n_preprocess,
      config_path,
      local_env,
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
struct ClientLocalStorage {
  base: BaseParams,
  local_hpt: LocalHashPrefixTable,
  preprocessed_queries: Option<Vec<PIRQueryParams>>,
}

fn main() {
  let Credential {
    username,
    password,
    n_preprocess,
    config_path,
    local_env,
  } = Credential::parse_from_cli_flags();

  let mut instance_urls: Vec<String> = REMOTE_INSTANCE_URLS_DEFAULT
    .iter()
    .map(|s| s.to_string())
    .collect();
  let mut buckets_total: usize = BUCKETS_TOTAL_DEFAULT;
  let mut buckets_per_instance = BUCKETS_PER_INSTANCE_DEFAULT;

  // if running locally, remote instances are taken from confguration file provided
  if local_env {
    let local_config = match parse_local_config(config_path.clone()) {
      Ok(v) => v,
      Err(err) => {
        panic!(
          "{}",
          format!("Error reading config file {}: {:?}", config_path, err)
        )
      }
    };

    instance_urls = match remote_instances_from_config(&local_config) {
      Ok(v) => v,
      Err(err) => panic!("{}", format!("Config file malformed: {:?}", err)),
    };

    buckets_total = match calculate_buckets_total(&local_config) {
      Ok(v) => v,
      Err(err) => panic!(
        "{}",
        format!("Config file malformed (namely instances.shards): {:?}", err)
      ),
    };

    buckets_per_instance = local_config.buckets_per_instance;
  }

  // offline phase
  println!("******* OFFLINE PHASE *******");
  let bucket_id: usize =
    client_get_bucket_id(&username, HEX_PREFIX_LEN, buckets_total as u32);
  let bucket_info =
    BucketInfo::new(bucket_id, instance_urls, buckets_per_instance);

  // If no params directory then create it
  if !Path::new(LOCAL_CACHE_DATA_DIR).is_dir() {
    std::fs::create_dir(LOCAL_CACHE_DATA_DIR).unwrap();
  }

  // attempt to read parameters locally or retrieve from server
  let bmd_path = format!("{}/{}", LOCAL_CACHE_DATA_DIR, bucket_id);
  let mut cls: ClientLocalStorage = if let Ok(s) =
    std::fs::read_to_string(&bmd_path)
  {
    println!(
      ">> Reading existing params from file for bucket: {}",
      bucket_id
    );
    serde_json::from_str(&s).unwrap()
  } else {
    println!(
        "> Retrieving params to check credential with username={} (bucket:{}) from {}",
        username, bucket_id, bucket_info.params_url
      ); // enforce bucket id, do not use cor.id

    let url = format!("{}/{}", bucket_info.params_url, bucket_id);

    let sor: ServerOfflineResponse =
      reqwest::blocking::get(url).unwrap().json().unwrap();

    sor.validate(sor.id).unwrap(); // skip validation check
    let result = sor.result.unwrap();

    println!("> Deserializing response");
    let (base, local_hpt) = result.deserialize_local_hpt().unwrap();
    ClientLocalStorage {
      base,
      local_hpt,
      preprocessed_queries: None,
    }
  };

  // online phase
  println!("******* LOCAL ONLINE PHASE *******");
  println!("> Checking credential locally");
  let data_to_hash = format!("{}{}", username, password);
  let indices = cls.local_hpt.get_indices(data_to_hash.as_bytes());
  let mut do_remote = true;
  if indices.is_empty() {
    println!(
      "> Credential not referenced as potential match, no query required"
    );
    println!("******* CREDENTIAL APPEARS SAFE *******");
    do_remote = false;
  }

  // do remote check if required
  if do_remote {
    println!("> Credential is a potential match, launching online query");

    // preprocess queries
    let preprocs = cls.preprocessed_queries;
    let query_params = if let Some(existing_qps) = preprocs {
      // attempt to use existing preprocessed data
      if existing_qps.len() < indices.len() {
        // preprocess minimum amount of extra queries
        println!(
          "> Have {} preprocessed queries, but need {}.",
          existing_qps.len(),
          indices.len()
        );
        let n = n_preprocess - existing_qps.len();
        let extra_qps = preprocess_queries(cls.base.clone(), n);
        let mut qps: Vec<PIRQueryParams> = Vec::with_capacity(n_preprocess);
        qps.extend(existing_qps);
        qps.extend(extra_qps);
        qps
      } else {
        // use existing preprocessed data
        println!("> Using previously derived preprocessed query data.",);
        existing_qps
      }
    } else {
      println!("> Need to preprocess query parameters.",);
      // derive a whole new set of preprocessed queries
      let n = if n_preprocess < indices.len() {
        indices.len()
      } else {
        n_preprocess
      };
      preprocess_queries(cls.base.clone(), n)
    };

    println!("******* REMOTE ONLINE PHASE *******");
    let (params_to_use, unused_params) = query_params.split_at(indices.len());
    let (cm, cs) =
      client_prepare_queries(params_to_use, &indices, data_to_hash.as_bytes())
        .unwrap();

    let cor = ClientOnlineRequest::new(cm, bucket_id);
    let cor_json = serde_json::to_string(&cor).unwrap();
    println!("> Sending query to {}", bucket_info.query_url);

    let client = reqwest::blocking::Client::new();
    let sor: ServerOnlineResponse = client
      .post(bucket_info.query_url)
      .body(cor_json)
      .send()
      .unwrap()
      .json()
      .unwrap();

    println!("> Parsing results from server");
    // return response
    if client_process_output(&sor.result.unwrap(), &cs).unwrap() {
      println!("******* CREDENTIAL COMPROMISED *******");
    } else {
      println!("******* CREDENTIAL APPEARS SAFE *******");
    }

    // update preprocessed query data to remove used parameters
    cls.preprocessed_queries = Some(unused_params.to_vec());
  }

  // write local storage params back to file if not runing locally
  if !local_env {
    println!("> Update local storage data");
    let json = json!(cls);
    serde_json::to_writer(&std::fs::File::create(bmd_path).unwrap(), &json)
      .unwrap();
  }
}

fn preprocess_queries(
  base_params: BaseParams,
  n: usize,
) -> Vec<PIRQueryParams> {
  println!("> Deriving full set of parameters.",);
  let cbp = ClientBucketParams::from(base_params);
  println!("> Preprocessing {} queries.", n);
  client_preproc_n_queries(&cbp, n).unwrap()
}

fn parse_local_config(
  path: String,
) -> Result<Config, Box<dyn std::error::Error>> {
  let f = std::fs::File::open(path)?;
  let config: Config = serde_yaml::from_reader(f)?;

  Ok(config)
}

fn remote_instances_from_config(
  config: &Config,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
  let mut instances_url = vec![];

  for instance in &config.instances {
    instances_url.push(format!("0.0.0.0:{}", instance.port));
  }

  Ok(instances_url)
}

fn calculate_buckets_total(
  config: &Config,
) -> Result<usize, Box<dyn std::error::Error>> {
  Ok(config.buckets_per_instance * config.instances.len())
}
