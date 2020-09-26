# AutoRust [![Software License](https://img.shields.io/badge/license-MIT-brightgreen.svg)](LICENSE)

A command line app similar to [AutoRest](https://github.com/azure/autorest), but is written in Rust to generate Rust code. The goal is to be able to generate code from the documents in [Azure/azure-rest-api-specs/specification](https://github.com/Azure/azure-rest-api-specs/tree/master/specification). If we can figure out the Language Server Protocol for AutoRest, we hope to compile an AutoRest Extension as well.

## Buliding

The [rustfmt-nightly](https://github.com/rust-lang/rustfmt) dependency requires that a couple of environment variables be set.
``` sh
export CFG_RELEASE_CHANNEL=nightly
export CFG_RELEASE=nightly
cargo build
```

## Running
The command line args are a subset of those supported by `autorest`.

``` sh
cargo run -- --help
cargo run -- --input-file=../azure-rest-api-specs/specification/vmware/resource-manager/Microsoft.AVS/stable/2020-03-20/vmware.json
```

## Status

It is early days. The generated code is not finished. No binaries have been published. You will probably get panics trying out other specs.