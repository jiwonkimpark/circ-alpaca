//! Export circ R1cs to Spartan
use crate::target::r1cs::*;
use fxhash::{FxHashMap as HashMap};
use gmp_mpfr_sys::gmp::limb_t;
use libspartan::{Assignment, InputsAssignment, Instance, NIZKGens, VarsAssignment, NIZK};
use libspartan::scalar::Scalar;
use libspartan::scalar::pasta::fq::Bytes;
use merlin::Transcript;
use rug::Integer;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path};
use std::time::Instant;
use libspartan::transcript::Keccak256Transcript;

/// Hold Spartan variables
#[derive(Debug)]
pub struct Variable {
    sid: usize,
    value: [u8; 32],
}

impl Variable {
    pub fn sid(&self) -> usize { self.sid }

    pub fn value(&self) -> [u8; 32] { self.value }
}

// pub fn r1cs_with_prover_input<P: AsRef<Path>>(
//     p_path: P,
//     inputs_map: &HashMap<String, Value>,
// ) {
//     let prover_data: ProverData = read_prover_data::<_>(P).expect("failed to read prover data");
//
//     // check modulus
//     let f_mod = prover_data.r1cs.field.modulus();
//     let s_mod = Integer::from_str_radix(
//         "28948022309329048855892746252171976963363056481941647379679742748393362948097",
//         10,
//     )
//         .unwrap();
//     assert_eq!(
//         &s_mod, f_mod,
//         "\nR1CS has modulus \n{s_mod},\n but Spartan CS expects \n{f_mod}",
//     );
//
//     // add r1cs witness to values
//     let values = prover_data.extend_r1cs_witness(inputs_map);
//     prover_data.r1cs.check_all(&values);
//     assert_eq!(values.len(), prover_data.r1cs.vars.len());
//
//     // write r1cs
//     // let mut file = File::create("./circ-mastadon/alpaca/r1cs.json").unwrap();
//     // file.write_all(
//     //     serde_json::to_string(&prover_data.r1cs)
//     //         .expect("failed to serialize r1cs to json")
//     //         .as_bytes()
//     // ).expect("Failed to write r1cs to the file");
//
//     // write values
//     let mut file = File::create(format!("{}/circ-mastadon/alpaca/r1cs_values.json", DIR)).unwrap();
//     file.write_all(
//         serde_json::to_string(&values)
//             .expect("failed to serialize values to json")
//             .as_bytes()
//     ).expect("Failed to write values to the file");
// }

/// generate spartan proof
pub fn prove<P: AsRef<Path>>(
    p_path: P,
    inputs_map: &HashMap<String, Value>,
) -> io::Result<(NIZKGens, Instance, NIZK)> {
    let prover_data = read_prover_data::<_>(p_path)?;

    let mut now = Instant::now();
    println!("Converting R1CS to Spartan");
    let (inst, wit, inps, num_cons, num_vars, num_inputs) =
        spartan::r1cs_to_spartan(&prover_data, inputs_map);
    let mut elapsed = now.elapsed();
    println!("spartan::r1cs_to_spartan: {:.2?}", elapsed);


    println!("Proving with Spartan");
    assert_ne!(num_cons, 0, "No constraints");

    now = Instant::now();
    // produce public parameters
    println!("Producing public parameters");
    let gens = NIZKGens::new(num_cons, num_vars, num_inputs);
    elapsed = now.elapsed();
    println!("NIZKGens::new: {:.2?}", elapsed);

    now = Instant::now();
    // produce proof
    println!("Producing proof");
    let mut prover_transcript = Keccak256Transcript::new(b"nizk_example");
    let pf = NIZK::prove(&inst, wit, &inps, &gens, &mut prover_transcript);
    println!("Proof produced");
    elapsed = now.elapsed();
    println!("NIZK::prove: {:.2?}", elapsed);

    Ok((gens, inst, pf))
}

/// verify spartan proof
pub fn verify<P: AsRef<Path>>(
    v_path: P,
    inputs_map: &HashMap<String, Value>,
    gens: &NIZKGens,
    inst: &Instance,
    proof: NIZK,
) -> io::Result<()> {
    let verifier_data = read_verifier_data::<_>(v_path)?;

    let values = verifier_data.eval(inputs_map);

    let mut inp = Vec::new();
    for v in &values {
        let scalar = int_to_scalar(&v.i());
        inp.push(scalar.to_bytes());
    }
    let inputs = InputsAssignment::new(&inp).unwrap();

    println!("Verifying with Spartan");
    let mut verifier_transcript = Keccak256Transcript::new(b"nizk_example");
    assert!(proof
        .verify(inst, &inputs, &mut verifier_transcript, gens)
        .is_ok());

    println!("Proof Verification Successful!");
    Ok(())
}

/// circ R1cs -> spartan R1CSInstance
pub fn r1cs_to_spartan(
    prover_data: &ProverData,
    inputs_map: &HashMap<String, Value>,
) -> (Instance, Assignment, Assignment, usize, usize, usize) {
    // spartan format mapper: CirC -> Spartan
    let mut wit = Vec::new();
    let mut inp = Vec::new();
    let mut trans: HashMap<Var, usize> = HashMap::default(); // Circ -> spartan ids
    let mut itrans: HashMap<usize, Var> = HashMap::default(); // spartan ids -> Circ

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
    // prover_data.r1cs.check_all(&values);

    assert_eq!(values.len(), prover_data.r1cs.vars.len());

    for var in prover_data.r1cs.vars.iter() {
        assert!(matches!(var.ty(), VarType::Inst | VarType::FinalWit));
        if let VarType::FinalWit = var.ty() {
            // witness
            let id = wit.len();
            itrans.insert(id, *var);
            trans.insert(*var, id);
            let val = values.get(var).expect("missing R1CS value");
            wit.push(int_to_scalar(&val.i()).to_bytes());
        }
    }

    let const_id = wit.len();

    for var in prover_data.r1cs.vars.iter() {
        assert!(matches!(var.ty(), VarType::Inst | VarType::FinalWit));
        if let VarType::Inst = var.ty() {
            // input
            let id = wit.len() + 1 + inp.len();
            itrans.insert(id, *var);
            trans.insert(*var, id);
            let val = values.get(var).expect("missing R1CS value");
            inp.push(int_to_scalar(&val.i()).to_bytes());
        }
    }

    let num_vars = wit.len();
    let num_inputs = inp.len();
    println!("# of variables (witnesses): {}", num_vars);
    println!("# of inputs: {}", num_inputs);
    println!("prover_data.r1cs.vars.len(): {}", wit.len() + inp.len());
    assert_eq!(wit.len() + inp.len(), prover_data.r1cs.vars.len());

    let assn_witness = VarsAssignment::new(&wit).unwrap();
    let assn_inputs = InputsAssignment::new(&inp).unwrap();

    // circuit
    let mut m_a: Vec<(usize, usize, [u8; 32])> = Vec::new();
    let mut m_b: Vec<(usize, usize, [u8; 32])> = Vec::new();
    let mut m_c: Vec<(usize, usize, [u8; 32])> = Vec::new();

    let mut i = 0; // constraint #
    for (lc_a, lc_b, lc_c) in prover_data.r1cs.constraints.iter() {
        // circ Lc (const, monomials <Integer>) -> Vec<Variable>
        let a = lc_to_v(lc_a, const_id, &trans);
        let b = lc_to_v(lc_b, const_id, &trans);
        let c = lc_to_v(lc_c, const_id, &trans);

        // constraint # x identifier (vars, 1, inp)
        for Variable { sid, value } in a {
            m_a.push((i, sid, value));
        }
        for Variable { sid, value } in b {
            m_b.push((i, sid, value));
        }
        for Variable { sid, value } in c {
            m_c.push((i, sid, value));
        }

        i += 1;
    }

    let num_cons = i;
    println!("# of constraints: {}", num_cons);

    let inst = Instance::new(num_cons, num_vars, num_inputs, &m_a, &m_b, &m_c).unwrap();

    // check if the instance we created is satisfiable
    let res = inst.is_sat(&assn_witness, &assn_inputs);
    assert!(res.unwrap());

    (
        inst,
        assn_witness,
        assn_inputs,
        num_cons,
        num_vars,
        num_inputs,
    )
}

// works fine with changing a field representation (Integer) to Fq (Scalar)
pub fn int_to_scalar(i: &Integer) -> Scalar {
    let mut accumulator = Scalar::zero();
    let limb_bits = (std::mem::size_of::<limb_t>() as u64) << 3;
    assert_eq!(limb_bits, 64);

    let two: u64 = 2;
    let mut m = Scalar::from(two.pow(63));
    m *= Scalar::from(two);

    // as_ref yields a least-significant-first array.
    for digit in i.as_ref().iter().rev() {
        accumulator *= m;
        accumulator += Scalar::from(*digit);
    }
    accumulator
}

// circ Lc (const, monomials <Integer>) -> Vec<Variable>
pub fn lc_to_v(lc: &Lc, const_id: usize, trans: &HashMap<Var, usize>) -> Vec<Variable> {
    let mut v: Vec<Variable> = Vec::new();

    for (k, m) in &lc.monomials {
        let scalar = int_to_scalar(&m.i());

        let var = Variable {
            sid: *trans.get(k).unwrap(),
            value: scalar.to_bytes(),
        };
        v.push(var);
    }
    if lc.constant.i() != 0 {
        let scalar = int_to_scalar(&lc.constant.i());
        let var = Variable {
            sid: const_id,
            value: scalar.to_bytes(),
        };
        v.push(var);
    }
    v
}

/// write prover and verifier data to file
pub fn write_data<P1: AsRef<Path>, P2: AsRef<Path>>(
    p_path: P1,
    v_path: P2,
    p_data: &ProverData,
    v_data: &VerifierData,
) -> io::Result<()> {
    write_prover_data(p_path, p_data)?;
    write_verifier_data(v_path, v_data)?;
    Ok(())
}

fn write_prover_data<P: AsRef<Path>>(path: P, data: &ProverData) -> io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    bincode::serde::encode_into_std_write(&data, &mut file, bincode::config::legacy()).unwrap();
    Ok(())
}

pub fn read_prover_data<P: AsRef<Path>>(path: P) -> io::Result<ProverData> {
    let mut file = BufReader::new(File::open(path)?);
    let data: ProverData = bincode::serde::decode_from_std_read(&mut file, bincode::config::legacy()).unwrap();
    Ok(data)
}

fn write_verifier_data<P: AsRef<Path>>(path: P, data: &VerifierData) -> io::Result<()> {
    let mut file = BufWriter::new(File::create(path)?);
    bincode::serde::encode_into_std_write(&data, &mut file, bincode::config::legacy()).unwrap();
    Ok(())
}

pub fn read_verifier_data<P: AsRef<Path>>(path: P) -> io::Result<VerifierData> {
    let mut file = BufReader::new(File::open(path)?);
    let data: VerifierData = bincode::serde::decode_from_std_read(&mut file, bincode::config::legacy()).unwrap();
    Ok(data)
}
