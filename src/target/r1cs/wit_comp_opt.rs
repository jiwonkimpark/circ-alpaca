//! A multi-stage R1CS witness evaluator.

use std::fs::File;
use std::{fs, io};
use std::collections::BTreeMap;
use std::io::{BufReader, BufWriter, Write};
use std::ops::{Add, Mul};
use std::path::Path;
use std::time::Instant;
use crate::ir::term::*;
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use serde::{Deserialize, Serialize};

use log::trace;
use crate::target::r1cs::ProverData;
use crate::target::r1cs::wit_comp::StagedWitComp;

/// Evaluates a staged witness computation.
#[derive(Debug)]
pub struct StagedWitCompEvaluator<'a> {
    comp: &'a StagedWitComp,
    variable_values: HashMap<String, Value>,
    step_values: Vec<Value>,
    stages_evaluated: usize,
    outputs_evaluted: usize,
}

impl<'a> StagedWitCompEvaluator<'a> {
    /// Create an empty witness computation.
    pub fn new(comp: &'a StagedWitComp) -> Self {
        Self {
            comp,
            variable_values: Default::default(),
            step_values: Default::default(),
            stages_evaluated: Default::default(),
            outputs_evaluted: 0,
        }
    }
    /// Have all stages been evaluated?
    pub fn is_done(&self) -> bool {
        self.stages_evaluated == self.comp.stages.len()
    }
    fn eval_step(&mut self) -> (usize, String) { //
        let next_step_idx = self.step_values.len();
        assert!(next_step_idx < self.comp.steps.len());
        let op = &self.comp.steps[next_step_idx].0;
        let args: Vec<&Value> = self
            .comp
            .step_args(next_step_idx)
            .map(|i| &self.step_values[i])
            .collect();

        let args_map: HashMap<usize, &Value> = self
            .comp
            .step_args(next_step_idx)
            .map(|i| (i, &self.step_values[i]))
            .collect();
        let args_idx: Vec<usize> = args_map.iter().map(|(i, _)| i.clone()).collect();
        let value = eval_op(op, &args, &self.variable_values);
        let eval_info = format!(
            "{} on {:?}",
            op,
            args_idx
        );
        self.step_values.push(value);
        return (next_step_idx, eval_info)
    }

    fn eval_step_operation(&mut self) -> (usize, (Op, Vec<usize>)) {
        let next_step_idx = self.step_values.len();

        let op = &self.comp.steps[next_step_idx].0;
        let args_idx: Vec<usize> = self
            .comp
            .step_args(next_step_idx)
            .collect();

        let args: Vec<&Value> = self
            .comp
            .step_args(next_step_idx)
            .map(|i| &self.step_values[i])
            .collect();

        let value = eval_op(op, &args, &self.variable_values);

        self.step_values.push(value);

        (next_step_idx, (op.clone(), args_idx))
    }

    fn eval_step_with(&mut self, operations: &BTreeMap<usize, (Op, Vec<usize>)>) {
        for (i, operation) in operations.iter().enumerate() {
            println!("step {}", i);
            let (operator, operand_ids) = operation.1;
            let operands: Vec<&Value> = operand_ids.iter().map(|id| &self.step_values[*id]).collect();
            let value = eval_op(operator, &operands, &self.variable_values);
            self.step_values.push(value)
        }
    }

    /// Evaluate one stage.
    pub fn eval_stage(&mut self, inputs: HashMap<String, Value>) -> Vec<&Value> {
        // let eval_operations = Self::read_eval_operations().unwrap();

        println!("========== EVAL_STAGE ==========");
        let total_timer = Instant::now();
        trace!(
            "Beginning stage {}/{}",
            self.stages_evaluated,
            self.comp.stages.len()
        );
        debug_assert!(self.stages_evaluated < self.comp.stages.len());
        let stage = &self.comp.stages[self.stages_evaluated];
        let num_outputs = stage.num_outputs;
        for (k, v) in &inputs {
            trace!("Input {}: {}", k, v,);
        }
        self.variable_values.extend(inputs);

        let mut eval_step_operations: BTreeMap<usize, (Op, Vec<usize>)> = BTreeMap::default();

        if num_outputs > 0 {
            let max_step = (0..num_outputs)
                .map(|i| {
                    let new_output_i = i + self.outputs_evaluted;
                    self.comp.ouput_steps[new_output_i]
                })
                .max()
                .unwrap();
            while self.step_values.len() <= max_step {
                // let (idx, eval_step) = self.eval_step();
                // eval_step_operations.insert(idx, eval_step);
                let (id, operation) = self.eval_step_operation();
                eval_step_operations.insert(id, operation);
            }
            // self.eval_step_with(&eval_operations);
        }
        self.outputs_evaluted += num_outputs;
        self.stages_evaluated += 1;
        let mut out = Vec::new();
        for output_step in
            &self.comp.ouput_steps[self.outputs_evaluted - num_outputs..self.outputs_evaluted]
        {
            out.push(&self.step_values[*output_step]);
        }

        Self::write_eval_operations(&eval_step_operations).expect("write eval operations failed");

        // println!("eval_steps.size: {}", eval_step_operations.len());
        // println!("eval_stage elapsed: {:.2?}", total_timer.elapsed());
        out
    }

    fn write_eval_operations(eval_operations: &BTreeMap<usize, (Op, Vec<usize>)>) -> io::Result<()> {
        let file_path = if Path::new("EVAL.json").exists() { Path::new("EVAL_1.json") } else { Path::new("EVAL.json") };
        let mut file = BufWriter::new(File::create(file_path).unwrap());
        bincode::serde::encode_into_std_write(&eval_operations, &mut file, bincode::config::legacy()).unwrap();
        Ok(())
    }

    fn read_eval_operations() ->io::Result<BTreeMap<usize, (Op, Vec<usize>)>>{
        let mut total_operations: BTreeMap<usize, (Op, Vec<usize>)> = BTreeMap::default();

        let mut file = BufReader::new(File::open("EVAL.json")?);
        let mut operations: BTreeMap<usize, (Op, Vec<usize>)> = bincode::serde::decode_from_std_read(&mut file, bincode::config::legacy()).unwrap();
        total_operations.extend(operations);

        file = BufReader::new(File::open("EVAL_1.json")?);
        operations = bincode::serde::decode_from_std_read(&mut file, bincode::config::legacy()).unwrap();
        total_operations.extend(operations);

        Ok(total_operations)
    }

    fn eval_step_direct(&mut self) {
        let w = &self.variable_values["w"];
        self.step_values.push(w.clone());
        let z = &self.variable_values["z"];
        self.step_values.push(z.clone());

        let w_times_z = w.as_pf().clone().mul(z.as_pf());
        self.step_values.push(Value::Field(w_times_z.clone()));

        let y = &self.variable_values["y"];
        self.step_values.push(y.clone());
        let x = &self.variable_values["x"];
        self.step_values.push(x.clone());

        let y_times_x = y.as_pf().clone().mul(x.as_pf());
        self.step_values.push(Value::Field(y_times_x.clone()));

        let result = w_times_z.add(y_times_x);
        self.step_values.push(Value::Field(result));
    }
}

#[cfg(test)]
mod test {

    use rug::Integer;

    use super::*;
    use circ_fields::FieldT;

    fn mk_inputs(v: Vec<(String, Sort)>) -> HashMap<String, Sort> {
        v.into_iter().collect()
    }

    #[test]
    fn one_const() {
        let mut comp = StagedWitComp::default();
        let field = FieldT::from(Integer::from(7));
        comp.add_stage(mk_inputs(vec![]), vec![pf_lit(field.new_v(0))]);

        let mut evaluator = StagedWitCompEvaluator::new(&comp);

        let output = evaluator.eval_stage(Default::default());
        let ex_output: &[usize] = &[0];
        assert_eq!(output.len(), ex_output.len());
        for i in 0..ex_output.len() {
            assert_eq!(output[i], &Value::Field(field.new_v(ex_output[i])), "{i}");
        }

        assert!(evaluator.is_done());
    }

    #[test]
    fn many_const() {
        let mut comp = StagedWitComp::default();
        let field = FieldT::from(Integer::from(7));
        comp.add_stage(mk_inputs(vec![]), vec![pf_lit(field.new_v(0))]);
        comp.add_stage(
            mk_inputs(vec![]),
            vec![pf_lit(field.new_v(1)), pf_lit(field.new_v(4))],
        );
        comp.add_stage(mk_inputs(vec![]), vec![pf_lit(field.new_v(6))]);
        comp.add_stage(mk_inputs(vec![]), vec![pf_lit(field.new_v(0))]);

        let mut evaluator = StagedWitCompEvaluator::new(&comp);

        let output = evaluator.eval_stage(Default::default());
        let ex_output: &[usize] = &[0];
        assert_eq!(output.len(), ex_output.len());
        for i in 0..ex_output.len() {
            assert_eq!(output[i], &Value::Field(field.new_v(ex_output[i])), "{i}");
        }

        let output = evaluator.eval_stage(Default::default());
        let ex_output: &[usize] = &[1, 4];
        assert_eq!(output.len(), ex_output.len());
        for i in 0..ex_output.len() {
            assert_eq!(output[i], &Value::Field(field.new_v(ex_output[i])), "{i}");
        }

        let output = evaluator.eval_stage(Default::default());
        let ex_output: &[usize] = &[6];
        assert_eq!(output.len(), ex_output.len());
        for i in 0..ex_output.len() {
            assert_eq!(output[i], &Value::Field(field.new_v(ex_output[i])), "{i}");
        }

        let output = evaluator.eval_stage(Default::default());
        let ex_output: &[usize] = &[0];
        assert_eq!(output.len(), ex_output.len());
        for i in 0..ex_output.len() {
            assert_eq!(output[i], &Value::Field(field.new_v(ex_output[i])), "{i}");
        }

        assert!(evaluator.is_done());
    }

    #[test]
    fn vars_one_stage() {
        let mut comp = StagedWitComp::default();
        let field = FieldT::from(Integer::from(7));
        comp.add_stage(mk_inputs(vec![("a".into(), Sort::Bool), ("b".into(), Sort::Field(field.clone()))]),
                       vec![
                           leaf_term(Op::Var("b".into(), Sort::Field(field.clone()))),
                           term![Op::Ite; leaf_term(Op::Var("a".into(), Sort::Bool)), pf_lit(field.new_v(1)), pf_lit(field.new_v(0))],
                       ]);

        let mut evaluator = StagedWitCompEvaluator::new(&comp);

        let output = evaluator.eval_stage(
            vec![
                ("a".into(), Value::Bool(true)),
                ("b".into(), Value::Field(field.new_v(5))),
            ]
                .into_iter()
                .collect(),
        );
        let ex_output: &[usize] = &[5, 1];
        assert_eq!(output.len(), ex_output.len());
        for i in 0..ex_output.len() {
            assert_eq!(output[i], &Value::Field(field.new_v(ex_output[i])), "{i}");
        }

        assert!(evaluator.is_done());
    }

    #[test]
    fn vars_many_stages() {
        let mut comp = StagedWitComp::default();
        let field = FieldT::from(Integer::from(7));
        comp.add_stage(mk_inputs(vec![("a".into(), Sort::Bool), ("b".into(), Sort::Field(field.clone()))]),
                       vec![
                           leaf_term(Op::Var("b".into(), Sort::Field(field.clone()))),
                           term![Op::Ite; leaf_term(Op::Var("a".into(), Sort::Bool)), pf_lit(field.new_v(1)), pf_lit(field.new_v(0))],
                       ]);
        comp.add_stage(mk_inputs(vec![("c".into(), Sort::Field(field.clone()))]),
                       vec![
                           term![PF_ADD;
               leaf_term(Op::Var("b".into(), Sort::Field(field.clone()))),
               leaf_term(Op::Var("c".into(), Sort::Field(field.clone())))],
                           term![Op::Ite; leaf_term(Op::Var("a".into(), Sort::Bool)), pf_lit(field.new_v(1)), pf_lit(field.new_v(0))],
                           term![Op::Ite; leaf_term(Op::Var("a".into(), Sort::Bool)), pf_lit(field.new_v(0)), pf_lit(field.new_v(1))],
                       ]);

        let mut evaluator = StagedWitCompEvaluator::new(&comp);

        let output = evaluator.eval_stage(
            vec![
                ("a".into(), Value::Bool(true)),
                ("b".into(), Value::Field(field.new_v(5))),
            ]
                .into_iter()
                .collect(),
        );
        let ex_output: &[usize] = &[5, 1];
        assert_eq!(output.len(), ex_output.len());
        for i in 0..ex_output.len() {
            assert_eq!(output[i], &Value::Field(field.new_v(ex_output[i])), "{i}");
        }

        let output = evaluator.eval_stage(
            vec![("c".into(), Value::Field(field.new_v(3)))]
                .into_iter()
                .collect(),
        );
        let ex_output: &[usize] = &[1, 1, 0];
        assert_eq!(output.len(), ex_output.len());
        for i in 0..ex_output.len() {
            assert_eq!(output[i], &Value::Field(field.new_v(ex_output[i])), "{i}");
        }

        assert!(evaluator.is_done());
    }
}
