# Filecoin Proving Subsystem (FPS)

The **Filecoin Proving Subsystem** provides the storage proofs required by the Filecoin protocol. It is implemented entirely in Rust, as a series of partially inter-dependent crates – some of which export C bindings to the supported API. This decomposition into distinct crates/modules is relatively recent, and in some cases current code has not been fully refactored to reflect the intended eventual organization.

There are currently four different crates:

- [**Storage Proofs (`storage-proofs`)**](./storage-proofs)
    A library for constructing storage proofs – including non-circuit proofs, corresponding SNARK circuits, and a method of combining them.

    `storage-proofs` is intended to serve as a reference implementation for _**Proof-of-Replication**_ (**PoRep**), while also performing the heavy lifting for `filecoin-proofs`.

     Primary Components:
     -   **PoR** (**_Proof-of-Retrievability_**: Merkle inclusion proof)
     -   **DrgPoRep** (_Depth Robust Graph_ **_Proof-of-Replication_**)
     -   **ZigZagDrgPoRep** (implemented as a specialized **LayeredDrgPoRep**)
     -   **PoSt** (Proof-of-Spacetime)


- [**Filecoin Proofs (`filecoin-proofs`)**](./filecoin-proofs)
  A wrapper around `storage-proofs`, providing an FFI-exported API callable from C (and in practice called by [go-filecoin](https://github.com/filecoin-project/go-filecoin') via cgo). Filecoin-specific values of setup parameters are included here, and circuit parameters generated by Filecoin’s (future) trusted setup will also live here.

- [**Sector Base (`sector-base`)**](./sector-base)
  A sector database abstracting away underlying storage considerations. This abstraction will allow for alternate implementations mapping logical sectors to physical storage – facilitating both support for miner specialization, and configurable adaptation to a given miner’s physical hardware and preferences.

- [**Storage Backend (`storage-backend`)**](./storage-backend)
  The `storage-backend` crate is intended to contain abstractions and implementations of non-Filecoin-specific storage mechanisms require by `storage-proofs`. However, for the sake of simplicity, it is currently an empty placeholder.

    ![FPS crate dependencies](/img/fps-dependencies.png?raw=true)

## Design Notes

Earlier in the design process, we considered implementing what has become the **FPS** in Go – as a wrapper around potentially multiple SNARK circuit libraries. We eventually decided to use [bellman](https://github.com/zkcrypto/bellman) – a library developed by Zcash, which supports efficient pedersen hashing inside of SNARKs. Having made that decision, it was natural and efficient to implement the entire subsystem in Rust. We considered the benefits (self-contained codebase, ability to rely on static typing across layers) and costs (developer ramp-up, sometimes unwieldiness of borrow-checker) as part of that larger decision and determined that the overall project benefits (in particular ability to build on Zcash’s work) outweighed the costs.

We also considered whether the **FPS** should be implemented as a standalone binary accessed from [**`go-filecoin`**](https://github.com/filecoin-project/go-filecoin) either as a single-invocation CLI or as a long-running daemon process. Bundling the **FPS** as an FFI dependency was chosen for both the simplicity of having a Filecoin node deliverable as a single monolithic binary, and for the (perceived) relative development simplicity of the API implementation.

If at any point it were to become clear that the FFI approach is irredeemably problematic, the option of moving to a standalone **FPS** remains. However, the majority of technical problems associated with calling from Go into Rust are now solved, even while allowing for a high degree of runtime configurability. Therefore, continuing down the same path we have already invested in, and have begun to reap rewards from, seems likely.

## Install and configure Rust

**NOTE:** If you have installed `rust-fil-proofs` incidentally, as a submodule of `go-filecoin`, then you may already have installed Rust.

The instructions below assume you have independently installed `rust-fil-proofs` in order to test, develop, or experiment with it.

[Install Rust.](https://www.rust-lang.org/en-US/install.html)

Configure to use nightly:

```
> rustup default nightly
```

## Build

```
> cargo build --release --all
```

## Test

```
> cargo test --all
```

## Examples

```
> cargo build --all --examples --release
```

Running them

```
> ./target/release/examples/merklepor
> ./target/release/examples/drgporep
> ./target/release/examples/drgporep-vanilla
> ./target/release/examples/drgporep-vanilla-disk
```

## Benchmarks

```
> cargo bench --all
```

To benchmark the examples you can [bencher](src/bin/bencher.rs).

```
# build the script
> cargo build
# run the benchmarks
> ./target/debug/bencher
```

The results are written into the `.bencher` directory, as JSON files. The benchmarks are controlled through the [bench.config.toml](bench.config.toml) file.

Note: On macOS you need `gtime` (`brew install gnu-time`), as the built in `time` command is not enough.

## Logging

For better logging with backtraces on errors, developers should use `expects` rather than `expect` on `Result<T, E>` and `Option<T>`.

Developers can control `rust-fil-proofs` logging through environment variables:

-
  `RUST_PROOFS_LOG_JSON`

    Default: `false`

    Options: `true`, `false`

    This is used to enable or disable logging as JSON. If it is `true`, log entries will be sent to stdout as JSON. Otherwise, log entries will be sent to stdout as plain text.

-
  `RUST_PROOFS_MIN_LOG_LEVEL`

    Default: `4`

    Options: `1`, `2`, `3`, `4`, `5`, `6`

    This is used to filter log entries. All log entries at the specified level or below will be sent to stdout.

    | Logging Macro 	| Level Code 	| Description                                                                                                                                                                                     |
    |---------------	|------------	|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
    | crit!         	| 1          	| An error that should force shutdown of the application to prevent data loss (or further data loss). Reserved for situations in which there is guaranteed to have been data corruption or loss.  |
    | error!        	| 2          	| An error occurred, generally something you would consider asserting in a debug build.                                                                                                           |
    | warning!      	| 3          	| A warning often indicates an unexpected (but not fatal) state.                                                                                                                                  |
    | info!         	| 4          	| An informational message, often indicates the current program state.                                                                                                                            |
    | debug!        	| 5          	| A debug message, useful for debugging but too verbose to be turned on normally.                                                                                                                 |
    | trace!        	| 6          	| A message that will be printed a lot, useful for debugging program flow and will probably impact performance.                                                                                   |

## Memory Leak Detection

To run the leak detector against the FFI-exposed portion of libfilecoin_proofs,
simply run the FFI example with leak detection enabled. On a Linux machine, you
can run the following command:

```shell
RUSTFLAGS="-Z sanitizer=leak" cargo run --release --package filecoin-proofs --example ffi --target x86_64-unknown-linux-gnu
```

If using mac OS, you'll have to run the leak detection from within a Docker
container. After installing Docker, run the following commands to build and run
the proper Docker image and then the leak detector itself:

```shell
docker build -t foo -f ./Dockerfile-ci . && \
  docker run \
    -it \
    -e RUSTFLAGS="-Z sanitizer=leak" \
    --privileged \
    -w /mnt/crate \
    -v `pwd`:/mnt/crate -v $(TMP=$(mktemp -d) && mv ${TMP} /tmp/ && echo /tmp${TMP}):/mnt/crate/target \
    foo:latest \
    cargo run --release --package filecoin-proofs --example ffi --target x86_64-unknown-linux-gnu
```

## Generate Documentation

First, navigate to the `rust-fil-proofs` directory.
- If you installed `rust-fil-proofs` automatically as a submodule of `go-filecoin`:
```
> cd <go-filecoin-install-path>/go-filecoin/proofs/rust-fil-proofs
```

- If you cloned `rust-fil-proofs` manually, it will be wherever you cloned it:
```
> cd <install-path>/rust-fil-proofs
```

[Note that the version of `rust-fil-proofs` included in `go-filecoin` as a submodule is not always the current head of `rust-fil-proofs/master`. For documentation corresponding to the latest source, you should clone `rust-fil-proofs` yourself.]

Now, generate the documentation:
```
> cargo doc --all --no-deps
```

View the docs by pointing your browser at: `…/rust-fil-proofs/target/doc/proofs/index.html`.

---

## API Reference

The **FPS** is accessed from [**go-filecoin**](https://github.com/filecoin-project/go-filecoin) via FFI calls to its API, which is the union of the APIs of its constituents:

 The Rust source code serves as the source of truth defining the **FPS** APIs. View the source directly:

- [**filecoin-proofs**](https://github.com/filecoin-project/rust-fil-proofs/blob/master/filecoin-proofs/src/api/mod.rs)
- [**sector-base**](https://github.com/filecoin-project/rust-fil-proofs/blob/master/sector-base/README.md#api-reference).


Or better, generate the documentation locally (until repository is public). Follow the instructions to generate documentation above. Then navigate to:
- **Sector Base API:** `…/rust-fil-proofs/target/doc/sector_base/api/index.html`
- **Filecoin Proofs API:** `…/rust-fil-proofs/target/doc/filecoin_proofs/api/index.html`

- [Go implementation of filecoin-proofs API](https://github.com/filecoin-project/go-filecoin/blob/master/proofs/rustprover.go) and [associated interface structures](https://github.com/filecoin-project/go-filecoin/blob/master/proofs/interface.go).
- [Go implementation of sector-base API](https://github.com/filecoin-project/go-filecoin/blob/master/proofs/disk_backed_sector_store.go).

## Contributing

See [Contributing](CONTRIBUTING.md)

## License

The Filecoin Project is dual-licensed under Apache 2.0 and MIT terms:

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
