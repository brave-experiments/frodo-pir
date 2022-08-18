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
