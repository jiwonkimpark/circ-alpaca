# ZSharp Implementation for Mastadon

## Running 

### Compile and Run the ZSharp Interpreter
More specific documentation in /README_zsharp.md

1. Install dependencies by running an appropriate script from `scripts/dependencies_*`
2. Build ZSharp interpreter cli : `cargo build --release --features r1cs,zok,spartan --example zk`
   After building as above, `target/release/examples/zk` will have been
   generated.
