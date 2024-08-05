use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::time::Instant;
use circ_fields::FieldV;
use fxhash::FxHashMap;
use libspartan::{Assignment, InputsAssignment, Instance, NIZK, NIZKGens, VarsAssignment};
use libspartan::scalar::pasta::fq::Bytes;
use merlin::Transcript;
use rug::Integer;
use serde::{Deserialize, Serialize};
use crate::ir::term::Value;
use crate::target::r1cs::{ProverData, R1csFinal, spartan, Var, VarType, VerifierData, wit_comp};
use crate::target::r1cs::spartan::{int_to_scalar};
use crate::target::r1cs::wit_comp::StagedWitComp;

#[derive(Debug, Serialize, Deserialize)]
pub struct SpartanInstance {
    num_cons: usize,
    num_wit: usize,
    num_inp: usize,
    m_a: Vec<(usize, usize, [u8; 32])>,
    m_b: Vec<(usize, usize, [u8; 32])>,
    m_c: Vec<(usize, usize, [u8; 32])>,
}

pub fn r1cs_values(
    r1cs: &R1csFinal,
    inputs_map: &FxHashMap<String, Value>,
) -> io::Result<FxHashMap<Var, FieldV>> {
    println!("========== CIRC - R1CS - SPARTAN - GET CIRCUIT VALUES ==========");
    let total_timer = Instant::now();

    let mut timer = Instant::now();
    let precompute: StagedWitComp = read_precompute::<_>("/Users/jiwonkim/research/tmp/Mastadon/IVC_PRECOMPUTE").expect("failed to read precompute data");
    let mut elapsed = timer.elapsed();
    println!("read prover precompute: {:.2?}", elapsed);

    timer = Instant::now();
    // check modulus
    let f_mod = r1cs.field.modulus();
    let s_mod = Integer::from_str_radix(
        "28948022309329048855892746252171976963363056481941647379679742748393362948097",
        10,
    )
        .unwrap();
    assert_eq!(
        &s_mod, f_mod,
        "\nR1CS has modulus \n{s_mod},\n but Spartan CS expects \n{f_mod}",
    );
    elapsed = timer.elapsed();
    println!("check modulus: {:.2?}", elapsed);

    // add r1cs witness to values
    timer = Instant::now();
    let values = r1cs.extend_r1cs_witness(&precompute, inputs_map);
    r1cs.check_all(&values);
    assert_eq!(values.len(), r1cs.vars.len());
    elapsed = timer.elapsed();
    println!("generate r1cs witness values time: {:.2?}", elapsed);

    let total_elapsed = total_timer.elapsed();
    println!("total circ r1cs values time: {:.?}", total_elapsed);
    println!("==============================");

    Ok(values)
}

pub fn write_precompute<P: AsRef<Path>>(path: P, data: &StagedWitComp) -> io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    bincode::serde::encode_into_std_write(&data, &mut file, bincode::config::legacy()).unwrap();
    Ok(())
}

pub fn read_precompute<P: AsRef<Path>>(path: P) -> io::Result<wit_comp::StagedWitComp> {
    let mut file = BufReader::new(File::open(path)?);
    let data: StagedWitComp = bincode::serde::decode_from_std_read(&mut file, bincode::config::legacy()).unwrap();
    Ok(data)
}


pub fn prove(
    prover_data: &ProverData,
    inputs_map: &FxHashMap<String, Value>,
    gens: &NIZKGens,
    inst: &Instance
) -> io::Result<NIZK> {
    let mut now = Instant::now();
    println!("Converting R1CS to Spartan");
    let (witnesses, inputs) = spartan_witnesses_and_inputs(prover_data, inst, inputs_map);
    let mut elapsed = now.elapsed();
    println!("spartan::r1cs_to_spartan: {:.2?}", elapsed);


    println!("Proving with Spartan");
    now = Instant::now();
    let mut prover_transcript = Transcript::new(b"nizk_example");
    let pf = NIZK::prove(inst, witnesses, &inputs, gens, &mut prover_transcript);
    elapsed = now.elapsed();
    println!("NIZK::prove: {:.2?}", elapsed);

    Ok(pf)
}

pub fn verify(
    verifier_data: &VerifierData,
    inputs_map: &FxHashMap<String, Value>,
    gens: &NIZKGens,
    inst: &Instance,
    proof: NIZK,
) -> io::Result<()> {
    let values = verifier_data.eval(inputs_map);

    let mut inp = Vec::new();
    for v in &values {
        let scalar = spartan::int_to_scalar(&v.i());
        inp.push(scalar.to_bytes());
    }
    let inputs = InputsAssignment::new(&inp).unwrap();

    println!("Verifying with Spartan");
    let mut verifier_transcript = Transcript::new(b"nizk_example");
    assert!(proof
        .verify(inst, &inputs, &mut verifier_transcript, gens)
        .is_ok());

    println!("Proof Verification Successful!");
    Ok(())
}

pub fn preprocess_spartan(prover_data: &ProverData) -> io::Result<(NIZKGens, Instance)> {
    let mut trans: FxHashMap<Var, usize> = HashMap::default(); // Circ -> spartan ids

    let mut id = 0;

    for var in prover_data.r1cs.vars.iter() {
        assert!(matches!(var.ty(), VarType::Inst | VarType::Chall | VarType::FinalWit | VarType::RoundWit ));
        match var.ty() {
            VarType::FinalWit | VarType::RoundWit => {
                trans.insert(*var, id);
                id += 1;
            }
            _ => {}
        }
    }

    let num_wit = id;
    let num_inp = prover_data.r1cs.vars.len() - id;
    id += 1; // for 1 in Z

    for var in prover_data.r1cs.vars.iter() {
        assert!(matches!(var.ty(), VarType::Inst | VarType::Chall | VarType::FinalWit | VarType::RoundWit ));
        match var.ty() {
            VarType::Inst | VarType::Chall => {
                trans.insert(*var, id);
                id += 1;
            }
            _ => {}
        }
    }

    assert_eq!(id, prover_data.r1cs.vars.len() + 1);

    let const_id = num_wit;
    // circuit
    let mut m_a: Vec<(usize, usize, [u8; 32])> = Vec::new();
    let mut m_b: Vec<(usize, usize, [u8; 32])> = Vec::new();
    let mut m_c: Vec<(usize, usize, [u8; 32])> = Vec::new();

    let mut i = 0; // constraint #
    for (lc_a, lc_b, lc_c) in prover_data.r1cs.constraints.iter() {
        // circ Lc (const, monomials <Integer>) -> Vec<Variable>
        let a = spartan::lc_to_v(lc_a, const_id, &trans);
        let b = spartan::lc_to_v(lc_b, const_id, &trans);
        let c = spartan::lc_to_v(lc_c, const_id, &trans);

        // constraint # x identifier (vars, 1, inp)
        for variable in a {
            m_a.push((i, variable.sid(), variable.value()));
        }
        for variable in b {
            m_b.push((i, variable.sid(), variable.value()));
        }
        for variable in c {
            m_c.push((i, variable.sid(), variable.value()));
        }

        i += 1;
    }
    let num_cons = i;
    assert_ne!(num_cons, 0, "No constraints");

    let gens = NIZKGens::new(num_cons, num_wit, num_inp);
    let inst = Instance::new(num_cons, num_wit, num_inp, &m_a, &m_b, &m_c).unwrap();

    Ok((gens, inst))
}

pub fn read_preprocessed_spartan<P1: AsRef<Path>, P2: AsRef<Path>>(gens_path: P1, inst_path: P2) -> io::Result<(NIZKGens, Instance)> {
    let mut file = BufReader::new(File::open(gens_path)?);
    let gens: NIZKGens = bincode::serde::decode_from_std_read(&mut file, bincode::config::legacy()).unwrap();

    file = BufReader::new(File::open(inst_path)?);
    let inst: Instance = bincode::serde::decode_from_std_read(&mut file, bincode::config::legacy()).unwrap();

    Ok((gens, inst))
}

pub fn write_preprocessed_spartan<P1: AsRef<Path>, P2: AsRef<Path>>(gens_path: P1, inst_path: P2, p_data: &ProverData) -> io::Result<()> {
    let (gens, inst) = preprocess_spartan(p_data).unwrap();

    let mut file = BufWriter::new(File::create(gens_path)?);
    bincode::serde::encode_into_std_write(&gens, &mut file, bincode::config::legacy()).unwrap();

    file = BufWriter::new(File::create(inst_path)?);
    bincode::serde::encode_into_std_write(&inst, &mut file, bincode::config::legacy()).unwrap();

    Ok(())
}

fn spartan_witnesses_and_inputs(
    prover_data: &ProverData,
    inst: &Instance,
    inputs_map: &FxHashMap<String, Value>,
) -> (Assignment, Assignment) {
    let mut wit = Vec::new();
    let mut inp = Vec::new();

    // check modulus
    let f_mod = prover_data.r1cs.field.modulus();
    let s_mod = Integer::from_str_radix(
        "28948022309329048855892746252171976963363056481941647379679742748393362948097",
        10,
    )
        .unwrap();
    assert_eq!(
        &s_mod, f_mod,
        "\nR1CS has modulus \n{s_mod},\n but Spartan CS expects \n{f_mod}",
    );

    let values = prover_data.extend_r1cs_witness(inputs_map);
    prover_data.r1cs.check_all(&values);

    for var in prover_data.r1cs.vars.iter() {
        let val = values.get(var).expect("missing R1CS value");
        match var.ty() {
            VarType::Inst => { inp.push(int_to_scalar(&val.i()).to_bytes()) }
            VarType::FinalWit => { wit.push(int_to_scalar(&val.i()).to_bytes()) }
            _ => { panic!("not supported var type") }
        };
    }
    assert_eq!(wit.len() + inp.len(), prover_data.r1cs.vars.len());

    let assn_witness = VarsAssignment::new(&wit).unwrap();
    let assn_inputs = InputsAssignment::new(&inp).unwrap();

    // check if the instance we created is satisfiable
    let res = inst.is_sat(&assn_witness, &assn_inputs);
    assert!(res.unwrap());

    (assn_witness, assn_inputs)
}