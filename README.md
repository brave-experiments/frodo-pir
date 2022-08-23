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
