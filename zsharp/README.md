# ZSharp Implementation for Mastadon

## Running 

### Compile and Run the ZSharp Interpreter
More specific documentation in /README_zsharp.md

1. Install dependencies by running an appropriate script from `scripts/dependencies_*`
2. Build ZSharp interpreter cli : `cargo build --release --example zxi --no-default-features --features smt,zok`
   After building as above, `target/release/examples/zxi` will have been
   generated.

### Run a ZSharp program
You can run ZSharp program by specifying the interpreter and the program's path.
For example:
`./target/release/examples/zxi ./zsharp/curves/test.zok`
