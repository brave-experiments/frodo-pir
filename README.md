# FrodoPIR

![FrodoPIR workflow](https://github.com/brave-experiments/frodo-pir/actions/workflows/rust.yml/badge.svg)

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

The source code can be built, tested, and benchmarked using [Docker](#using-docker).

In order to [natively](#native) build, run, test and benchmark the library, you will need the following:

```
  Rust >= 1.61.0
  Cargo
  Make
  Python3 >= 3.9.7
```

To obtain our performance numbers as reported in our paper, we run our benchmarks on an AWS EC2 c5n.metal instance.

## Quickstart

### Local

#### Building

To install the latest version of Rust, use the following command (you can also check how to install on the [Rust documentation](https://www.rust-lang.org/tools/install)):

```
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

To build the library, run:

```
  make build
```

#### Testing

To run the tests:

```
  make test
```

We test:

* A client and server workflow when using FrodoPIR (10 times).
* A test to check that the library fails if parameters are reused.

If all test build and run correctly, you should see an `ok` next to them.

**Note**: Occasionally, one of the tests will fail with a `thread 'api::tests::client_query_to_server_10_times' panicked at 'assertion failed: (left == right)` error. This is due to the usage of specific parameters for testing and can be safely ignored.

#### Documentation

To view documentation (in a web browser manner):

```
  make docs
```

#### Benchmarking

To run a specific set of benchmarks, run (note the this process is slow. On average, it takes 12 minutes):

```
  make bench
```

This command will execute client query benchmarks and Database generation benchmarks (for more details, see the `benches/bench.rs` file).

To run all benchmarks (note that this process is very slow, it takes around 30 minutes):

```
  make bench-all
```

This command will execute client query benchmarks and Database generation benchmarks for 16, 17, 18, 19 and 20 Number of DB items (log(m)). The results of these benchmarks can be found on Table 6 of our paper.

In order to see the results of the benchmarks, navigate to the `benchmarks-x.txt` file.

To interpret it in regards to Table 6 of our paper: `client query prepare` corresponds to the `Client query` row, `server response compute` corresponds to the `Server response` row, `client parse server response` corresponds to the `Client output` row, `generate db and params`, corresponds to the `Database preprocessing` row, `derive LHS from seed` corresponds to the `Client derive matrix` row, and `create client query params` corresponds to `Client query preprocessing` row.

![Performance numbers for FrodoPIR](/images/performance.png "Performance numbers for FrodoPIR")

If all benches build and run correctly, you should see an `Finished ... benchmarks` under them.
We use [Criterion](https://bheisler.github.io/criterion.rs/book/index.html) for benchmarking.
If you want to see and have explanations of the benchmarks, you can locally open `target/criterion/report/index.html` in your browser.

**Note**: When running the benches, a warning might appear ``Warning: Unable to complete 10 samples in 100.0s. You may wish to increase target time to 486.6s.``. If you want to silence the warning, you can change line 30 of `benches/bech.rs` file to 500 or more. Note that this will make the running of benches slower.

### Using Docker

The same notes as in the previous section also apply here.

Build Docker image:

```
docker build -t frodo-pir .
```

Build and run tests:

```
docker run --rm frodo-pir
```

Run Docker image interactively (from here, you can run any of the `make` commands below):

```
docker run --rm -it --entrypoint /bin/bash frodo-pir
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

The `data` directory contains a python script used to generate the needed data for testing.
The `benchs` directory contains a script used to benchmark the library.
The `pi-rs-cli-utils` directory contains files of 'utility' functionality.

### How to use

An easy way to see how to use the library can be found on the tests on the `api.rs` file. For full code documentation of the code, run `make docs`.

The following code is a copy of the aforementioned test. It exemplifies how FrodoPIR can be used by mocking both a server and a client.
First, the server generates all the needed parameters in relationship to a mocked random database.
Then, the client downloads said parameters and prepares queries (as part of the for-loop).
Then, both server and client interact such that the server outputs the correct response.
In order to run this example, one can run the tests.

```rust
  use frodo_pir::api::*;
  fn client_query_e2e() {
    /* Preprocessing performed by the server */

    // The LWE dimension to use
    let lwe_dim = 1572;
    // The number of rows in the database
    let m = 2u32.pow(16) as usize;
    // The length of each element in the database
    let ele_size = 2u32.pow(13) as usize;
    // The number of plaintext bits to use in each matrix element
    //   - 10 bits, for 16 ≤ log2(m) ≤ 18
    //   - 9 bits, for log2(m) ≤ 20
    // see Section 5 of paper for full details
    let plaintext_bits = 10usize;
    // Generates a random database
    let db_eles = generate_db_eles(m, (ele_size + 7) / 8);
    let db = Shard::from_base64_strings(&db_eles, lwe_dim, m, ele_size, plaintext_bits);
    // Parameters used by the server
    let base_params = db.get_base_params();

    // Public parameters downloaded by the client
    let common_params = CommonParams::from(base_params);

    /* Run client queries */
    for i in 0..10 {
      // Preprocess client queries before knowing query index (can be done offline)
      let mut query_params = QueryParams::new(&common_params, base_params);
      // Generate client query for index `i` of database
      let query = query_params.prepare_query(i);
      // Server response to query
      let d_resp = db.respond(&query).unwrap();
      // Client post-processing of server response
      let resp: Response = bincode::deserialize(&d_resp).unwrap();
      let output = resp.parse_output_as_base64(&query_params);
      // Check that client output matches row `i` of server DB.
      assert_eq!(output, db_eles[i]);
    }
  }
```
### Tests

We have two big tests that the library executes:

1. `client_query_to_server_10_times()` test which executes the client-to-server functionality:
   the client asks for an item in the database and the server is able to privately return it.
   The test asserts that the returned item is indeed the correct item in the database.
   It executes a for loop 10 times.
2. `client_query_to_server_attempt_params_reuse` test which executes the client-to-server
   functionality one time. It asserts that once parameters for a query are used, they
   are marked as so, and cannot be reused.

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
