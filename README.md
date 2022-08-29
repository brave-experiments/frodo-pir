# FrodoPIR

An implementation of the FrodoPIR Private Information Retrieval scheme. Find the details over [our eprint paper](https://eprint.iacr.org/2022/981.pdf).

We design *FrodoPIR*, a highly configurable, stateful, single-server Private Information Retrieval (PIR)
scheme that involves an offline phase that is completely client-independent. Coupled with small online
overheads, it leads to much smaller amortized financial costs on the server-side than previous approaches.
In terms of performance for a database of 1 million KB elements, FrodoPIR requires <1 second for
responding to a client query, has a server response size blow-up factor of > 3.6x, and financial costs are
~$1 for answering client queries. Our experimental analysis is built upon a simple, non-optimized
Rust implementation, illustrating that FrodoPIR is eminently suitable for large practical deployments.

*Warning*: This code is a research prototype. Do not use it in production.

## Requirements

In order to build, run, test and benchmark the library, you will need:

```
  Rust >= 1.61.0
  Cargo
  Make
  Python3
```

However, if you want to run the client-server interactions in FrodoPIR, you will need:

```
  Make
  Docker
```

## Quickstart

To build the library, run:

```
  make build
```

To run the tests:

```
  make test
```

To run the benchmarks (note that this process is very slow):

```
  make bench-all
```

## Overview

### FrodoPIR main functionality

The `src` folder contains the main *FrodoPIR* functionality. In particular:
  * `api.rs`: provides the main *FrodoPIR* API:
    * To read and generate the appropriate parameters: `from_json_file` (from a file) or `from_base64_strings` (from strings).
      (This corresponds to the 'Server setup' and 'Server preprocessing' phases from the paper).
    * To prepare and create the client query: `prepare_query` (this corresponds to the 'Client query generation' phase from the paper).
    * To analyse the client query and create the server response: `respond` (this corresponds to the 'Server response' phase from the paper).
  * The `db.rs` file contains the main functionality to be used for database processing.
  * The `util.rs` file contains utility functions.

### How to use

An easy way to see how to use the library can be found on the tests on the `api.rs` file:

```
    fn client_query_e2e() {
        let lwe_dim = 512;
        let m = 2u32.pow(12) as usize;
        let ele_size = 2u32.pow(8) as usize;
        let plaintext_bits = 12usize;
        let db_eles = generate_db_eles(m, (ele_size + 7) / 8);
        let shard = Shard::from_base64_strings(&db_eles, lwe_dim, m, ele_size, plaintext_bits);
        let base_params = shard.get_base_params();
        let common_params = CommonParams::from(base_params);
        #[allow(clippy::needless_range_loop)]
        for i in 0..10 {
            let mut query_params = QueryParams::new(&common_params, base_params);
            let query = query_params.prepare_query(i);
            let d_resp = shard.respond(&query).unwrap();
            let resp: Response = bincode::deserialize(&d_resp).unwrap();
            let output = resp.parse_output_as_base64(&query_params);
            assert_eq!(output, db_eles[i]);
        }
    }
```


## Running the Docker client-server local environment

In order to facilitate running FrodoPIR servers with arbitrary DB contents and performing private queries, we automated the build, database content preparation, server initiation and query steps. The steps to configure and run the local environment are the following:

1. **Define configurations**: the configurations for the server and clients are set in `data/local-configs.yml`; 
2. **Prepare the database contents**: given a file with pairs of `username:password`, this step processes and stores in disk the values that will be loaded by the database instances;
3. **Build server and client docker images**
4. **Start the server instances**: based on the configuration file, there may be more than one server instance required. Each server instance loads one or many credential buckets (prepared in step 2.);
5. **Client query**: once the server instances are running on docker containers with the database contents loaded (step 4.), we can perform queries against the server instances.

The local environment was configured to be as close to a production environment as possible. Thus, we can run several instances of the server, each responsible for a set of credential buckets. In production, this design helps to improve scalability, costs and robustness. Figure 1. depicts the schema of the local environment.


**TODO(@gpestana): center and format size of the image**

![frodo-pir-local](https://user-images.githubusercontent.com/1398860/187131871-2eabfed2-8757-462d-b6b5-008b7238ecb5.png)

Figure 1. Schema of the local environment with user making query against bucket 2.

For ease of use, all the required steps to configure, prepare, run the servers and issue queries against the local database only requires three commands and Docker installed. The communication between the client and server docker containers is performed through HTTP endpoints.

### Steps to build, run and query in the local environment:

```bash

 $ make prepare

 $ make run-server

 $ USERNAME=username@mail.com PWD=leaked_pwd query
```

### Configuration file

The local environment configuration file can be found at [./data/local-env/local-configs.yml](./data/local-env/local-configs.yml). The bucket preparation step, as well as the server and client processes will read the configuration to ensure that the client and server are in sync. The configuration file can be edited before the `prepare` run step. Note that the relative paths in the configuration file are relative to the project root path.

The contents of the configuration file is the following:

```yaml
db_content_path: "./data/local-env/creds"
buckets_path: "./data/local-env/local_buckets"
oprf_key: "VBiS3Zlp4UjLXLf9nw4GtU0j5LVfA9T+0u31skECPAY="
buckets_per_instance: 4
instances:
   - port: 8081
     shards: 0-3
   - port: 8082
     shards: 4-7
   - port: 8083
     shards: 8-11
```

where,

- `db_content_path`: contains the raw database content which consists of credential pairs (one per line). See [./data/local-env/creds/0](./data/local-env/creds/0) as an example.
- `buckets_path`: sets the path where the processed buckets will be stored.
- `oprf_key`: is the OPRF key used by all instances.
- `buckets_per_instance`: sets how many buckets (i.e. shards) each instance will be hosting and serving through their HTTP endpoints.
- `instances`: is a list of configurations for each specific server instance that will run. For each instance, we define the i) port through which the docker container will be accepting HTTP connections; and ii) an interval indicating which shards the server instance will host.

When running the local environment with the configuration file above, the automation tools will start 3 docker containers running the FrodoPIR process. The container that listens under port `8081` expose two HTTP endpoints (`GET /params` and `GET /query`) and will be serving clients that want to query against shards `0, 1, 2, 3`. 

### Requirements
The local environment requires only Docker to run.   

## Citation

```
@misc{cryptoeprint:2022/949,
      author = {Alex Davidson and Gonçalo Pestana and Sofía Celi},
      title = {FrodoPIR: Simple, Scalable, Single-Server Private Information Retrieval},
      howpublished = {Cryptology ePrint Archive, Paper 2022/981},
      year = {2022},
      note = {\url{https://eprint.iacr.org/2022/981}},
      url = {https://eprint.iacr.org/2022/981}
}
```
