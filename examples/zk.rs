use std::io::BufReader;
use circ::cfg::{
    clap::{self, Parser, ValueEnum},
    CircOpt,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[cfg(feature = "bellman")]
use bls12_381::Bls12;
use libspartan::{Instance, NIZK, NIZKGens};
#[cfg(feature = "bellman")]
use circ::target::r1cs::{bellman::Bellman, mirage::Mirage, proof::ProofSystem};

#[cfg(feature = "spartan")]
use circ::ir::term::text::parse_value_map;
#[cfg(feature = "spartan")]
use circ::target::r1cs::spartan;

#[derive(Debug, Parser)]
#[command(name = "zk", about = "The CirC ZKP runner")]
struct Options {
    #[arg(long, default_value = "P")]
    prover_key: PathBuf,
    #[arg(long, default_value = "V")]
    verifier_key: PathBuf,
    #[arg(long, default_value = "pi")]
    proof: PathBuf,
    #[arg(long, default_value = "in")]
    inputs: PathBuf,
    #[arg(long, default_value = "pin")]
    pin: PathBuf,
    #[arg(long, default_value = "vin")]
    vin: PathBuf,
    #[arg(long, default_value = "groth16")]
    proof_impl: ProofImpl,
    #[arg(long)]
    action: ProofAction,
    #[command(flatten)]
    circ: CircOpt,
}

#[derive(PartialEq, Debug, Clone, ValueEnum)]
/// `Prove`/`Verify` execute proving/verifying in bellman separately
/// `Spartan` executes both proving/verifying in spartan
enum ProofAction {
    Prove,
    Verify,
    Spartan,
    SpartanProve,
    SpartanVerify,
    SpartanR1CS,
}

#[derive(PartialEq, Debug, Clone, ValueEnum)]
/// Whether to use Groth16 or Mirage
enum ProofImpl {
    Groth16,
    Mirage,
}

fn main() {
    env_logger::Builder::from_default_env()
        .format_level(false)
        .format_timestamp(None)
        .init();
    let opts = Options::parse();
    circ::cfg::set(&opts.circ);
    match (opts.action, opts.proof_impl) {
        #[cfg(feature = "bellman")]
        (ProofAction::Prove, ProofImpl::Groth16) => {
            println!("Proving");
            Bellman::<Bls12>::prove_fs(opts.prover_key, opts.inputs, opts.proof).unwrap();
        }
        #[cfg(feature = "bellman")]
        (ProofAction::Prove, ProofImpl::Mirage) => {
            println!("Proving");
            Mirage::<Bls12>::prove_fs(opts.prover_key, opts.inputs, opts.proof).unwrap();
        }
        #[cfg(feature = "bellman")]
        (ProofAction::Verify, ProofImpl::Groth16) => {
            println!("Verifying");
            assert!(
                Bellman::<Bls12>::verify_fs(opts.verifier_key, opts.inputs, opts.proof).unwrap(),
                "invalid proof"
            );
        }
        #[cfg(feature = "bellman")]
        (ProofAction::Verify, ProofImpl::Mirage) => {
            println!("Verifying");
            assert!(
                Mirage::<Bls12>::verify_fs(opts.verifier_key, opts.inputs, opts.proof).unwrap(),
                "invalid proof"
            );
        }
        #[cfg(not(feature = "bellman"))]
        (ProofAction::Prove | ProofAction::Verify, _) => panic!("Missing feature: bellman"),
        #[cfg(feature = "spartan")]
        (ProofAction::Spartan, _) => {
            let prover_input_map = parse_value_map(&std::fs::read(opts.pin).unwrap());
            let (gens, inst, proof) = spartan::prove(opts.prover_key, &prover_input_map).unwrap();

            let verifier_input_map = parse_value_map(&std::fs::read(opts.vin).unwrap());
            spartan::verify(opts.verifier_key, &verifier_input_map, &gens, &inst, proof).unwrap();
        }
        // #[cfg(feature = "spartan")]
        // (ProofAction::SpartanR1CS, _) => {
        //     let mut now = Instant::now();
        //
        //     let prover_input_map = parse_value_map(&std::fs::read(opts.pin).unwrap());
        //
        //     let mut elapsed = now.elapsed();
        //     println!("Elapsed for generating prover_input_map: {:.2?}", elapsed);
        //
        //     println!("Getting R1CS");
        //     spartan::r1cs_with_prover_input(opts.prover_key, &prover_input_map);
        //     println!("Successfully retrieved R1CS")
        // }
        // #[cfg(feature = "spartan")]
        // (ProofAction::SpartanProve, _) => {
        //     let prover_input_map = parse_value_map(&std::fs::read(opts.pin).unwrap());
        //
        //     println!("Spartan Proving");
        //     let (_gens, _inst, _proof) = spartan::prove(opts.prover_key, &prover_input_map).unwrap();
        //
        // }
        // #[cfg(feature = "spartan")]
        // (ProofAction::SpartanVerify, _) => {
        //     let proof = read_proof();
        //     let inst = read_instance();
        //     let gens = read_gens();
        //
        //     let verifier_input_map = parse_value_map(&std::fs::read(opts.vin).unwrap());
        //     println!("Spartan Verifying");
        //     spartan::verify(opts.verifier_key, &verifier_input_map, &gens, &inst, proof).unwrap();
        // }
        #[cfg(not(feature = "spartan"))]
        (ProofAction::Spartan, _) => panic!("Missing feature: spartan"),
    }
}
