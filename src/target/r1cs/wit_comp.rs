//! A multi-stage R1CS witness evaluator.

use crate::cfg::cfg_or_default;
use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter};
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use circ_fields::FieldV;
use crate::ir::term::*;
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use serde::{Deserialize, Serialize};

use log::trace;
use crate::ir::term::Value::Field;
// use crate::target::r1cs::eval_op::eval_op_with;

use std::time::Duration;

/// A witness computation that proceeds in stages.
///
/// In each stage:
/// * it takes a partial assignment
/// * it returns a vector of field values
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StagedWitComp {
    pub(crate) vars: HashSet<String>,
    pub stages: Vec<Stage>,
    pub steps: Vec<(Op, usize)>,
    pub(crate) step_args: Vec<usize>,
    pub ouput_steps: Vec<usize>,
    // we don't serialize the cache; it's just used during construction, and terms are expensive to
    // serialize.
    #[serde(skip)]
    term_to_step: TermMap<usize>,
}

/// Specifies a stage.
#[derive(Debug, Serialize, Deserialize)]
pub struct Stage {
    inputs: HashMap<String, Sort>,
    pub num_outputs: usize,
}

/// Builder interface
impl StagedWitComp {
    /// Add a new stage.
    #[allow(clippy::uninlined_format_args)]
    pub fn add_stage(&mut self, inputs: HashMap<String, Sort>, output_values: Vec<Term>) {
        let stage = Stage {
            inputs,
            num_outputs: output_values.len(),
        };
        for input in stage.inputs.keys() {
            debug_assert!(!self.vars.contains(input), "Duplicate input {}", input);
        }
        self.vars.extend(stage.inputs.keys().cloned());
        self.stages.push(stage);
        let already_have: TermSet = self.term_to_step.keys().cloned().collect();
        for t in PostOrderIter::from_roots_and_skips(output_values.clone(), already_have) {
            self.add_step(t);
        }
        for t in output_values {
            self.ouput_steps.push(*self.term_to_step.get(&t).unwrap());
        }
    }

    fn add_step(&mut self, term: Term) {
        debug_assert!(!self.term_to_step.contains_key(&term));
        let step_idx = self.steps.len();
        if let Op::Var(var) = term.op() {
            debug_assert!(self.vars.contains(&*var.name));
        }
        for child in term.cs() {
            let child_step = self.term_to_step.get(child).unwrap();
            self.step_args.push(*child_step);
        }
        self.steps.push((term.op().clone(), self.step_args.len()));
        self.term_to_step.insert(term, step_idx);
    }

    /// How many stages are there?
    pub fn stage_sizes(&self) -> impl Iterator<Item=usize> + '_ {
        self.stages.iter().map(|s| s.num_outputs)
    }

    /// How many inputs are there for this stage?
    pub fn num_stage_inputs(&self, n: usize) -> usize {
        self.stages[n].inputs.len()
    }
}

/// Evaluator interface
impl StagedWitComp {
    pub fn step_args(&self, step_idx: usize) -> impl Iterator<Item=usize> + '_ {
        assert!(step_idx < self.steps.len());
        let args_end = self.steps[step_idx].1;
        let args_start = if step_idx == 0 {
            0
        } else {
            self.steps[step_idx - 1].1
        };
        (args_start..args_end).map(move |step_arg_idx| self.step_args[step_arg_idx])
    }
}

/// Evaluates a staged witness computation.
#[derive(Debug)]
pub struct StagedWitCompEvaluator<'a> {
    comp: &'a StagedWitComp,
    variable_values: HashMap<String, Value>,
    step_values: Vec<Value>,
    stages_evaluated: usize,
    outputs_evaluted: usize,
    op_times: HashMap<(Op, Vec<Sort>), (Duration, usize)>,
    time_ops: bool,
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
            op_times: Default::default(),
            time_ops: cfg_or_default().ir.time_eval_ops,
        }
    }
    /// Have all stages been evaluated?
    pub fn is_done(&self) -> bool {
        self.stages_evaluated == self.comp.stages.len()
    }

    fn eval_step(&mut self) { //
        let next_step_idx = self.step_values.len();
        assert!(next_step_idx < self.comp.steps.len());
        let op = &self.comp.steps[next_step_idx].0;
        let step_values = &self.step_values;
        let op_times = &mut self.op_times;
        let args: Vec<&Value> = self
            .comp
            .step_args(next_step_idx)
            .map(|i| &step_values[i])
            .collect();
        let value = if self.time_ops {
            let start = std::time::Instant::now();
            let r = eval_op(op, &args, &self.variable_values);
            let duration = start.elapsed();
            let (ref mut dur, ref mut ct) = op_times
                .entry((op.clone(), args.iter().map(|v| v.sort()).collect()))
                .or_default();
            *dur += duration;
            *ct += 1;
            r
        } else {
            eval_op(op, &args, &self.variable_values)
        };

        trace!(
            "Eval step {}: {} on {:?} -> {}",
            next_step_idx,
            op,
            args,
            value
        );
        self.step_values.push(value);
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

    fn get_eq(&self, id: usize, operands: Vec<&Value>) -> String {
        format!("let id_{} = Value::Bool({}.as_bool() == {}.as_bool())", id, operands[0], operands[1])
    }
    // fn eval_step_with(&mut self, operations: &BTreeMap<usize, (Op, Vec<usize>)>) {
    //     let file = File::create("./eval_op.rs").unwrap();
    //     let mut writer = BufWriter::new(&file);
    //     writeln!(&mut writer, "use fxhash::{{FxHashMap as HashMap}};").expect("failed to write import");
    //     writeln!(&mut writer, "use crate::ir::term::*;").expect("failed to write import");
    //
    //     writeln!(&mut writer, "fn eval_op(variable_values: &HashMap<String, Value>) {{").expect("failed to write a function def");
    //     // eval_op_with(&self.variable_values);
    //     for (id, operation) in operations.iter().enumerate() {
    //         let (operator, operand_ids) = operation.1;
    //         let operands: Vec<&Value> = operand_ids.iter().map(|id| &self.step_values[*id]).collect();
    //         let value = eval_op(operator, &operands, &self.variable_values);
    //         if id < 100 {
    //             match operator {
    //                 Op::Var(n, _) => {
    //                     // println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                     writeln!(&mut writer, "let id_{} = variable_values.get(\"{}\").unwrap().clone();", id, n).expect("failed to write Op::Var");
    //                     // let id_3 = &self.variable_values.get("r").unwrap().clone();
    //                 }
    //                 Op::Eq => {
    //                     // println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                     writeln!(&mut writer, "let id_{} = Value::Bool({} == {});", id, format!("id_{}", operand_ids[0]), format!("id_{}", operand_ids[1])).expect("failed to write Op::Eq");
    //                 }
    //                 Op::Not | Op::Implies => {
    //                     println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                 }
    //                 Op::BoolNaryOp(op) => {
    //                     match op {
    //                         BoolNaryOp::And => {
    //                             println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                         }
    //                         BoolNaryOp::Xor => {
    //                             println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                         }
    //                         BoolNaryOp::Or => {
    //                             writeln!(&mut writer, "let mut operands = Vec::new();");
    //                             for operand_id in operand_ids {
    //                                 writeln!(&mut writer, "operands.push(id_{}.clone());", operand_id);
    //                             }
    //                             writeln!(&mut writer, "let mut result = false;");
    //                             writeln!(&mut writer, "for operand in operands {{ if operand {{ result = true }} }}");
    //                             writeln!(&mut writer, "let id_{} = result;", id);
    //                         }
    //                     }
    //                 }
    //                 Op::BvBit(_) | Op::BoolMaj | Op::BvConcat | Op::BvExtract(..) | Op::BvUnOp(_) | Op::BvSext(_) | Op::PfToBv(_) | Op::BvUext(_) => {
    //                     println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                 }
    //                 Op::Const(v) => {
    //                     match v.clone() {
    //                         Value::BitVector(bv) => {writeln!(&mut writer, "let id_{} bv = {}, {};", id, bv.uint(), bv.width()).expect("failed to write Op::Const");}
    //                         Value::F32(f) => {writeln!(&mut writer, "let id_{} f32 = {};", id, f).expect("failed to write Op::Const");}
    //                         Value::F64(f) => {writeln!(&mut writer, "let id_{} f64 = {};", id, f).expect("failed to write Op::Const");}
    //                         Value::Int(i) => {writeln!(&mut writer, "let id_{} int = {};", id, i).expect("failed to write Op::Const");}
    //                         Value::Field(f) => {writeln!(&mut writer, "let id_{} field = {};", id, f).expect("failed to write Op::Const");}
    //                         Value::Bool(b) => {writeln!(&mut writer, "let id_{} bool = {};", id, b).expect("failed to write Op::Const");}
    //                         Value::Array(a) => {
    //                             println!("{:?}", id);
    //                             if id == 30 {
    //                                 writeln!(&mut writer, "let mut map: BTreeMap<Value, Value> = BTreeMap::new();").expect("failed to write Op::Const");
    //                                 for (k, v) in a.map.iter() {
    //
    //                                     writeln!(&mut writer, "let mut v_map: BTreeMap<Value, Value> = BTreeMap::new();").expect("failed to write Op::Const");
    //                                     for (v_k, v_v) in v.as_array().map.iter() {
    //                                         println!("v_v.key_sort: {}", v_v.as_array().key_sort);
    //                                         println!("v_v default: {}", v_v.as_array().default);
    //                                         writeln!(&mut writer, "let mut v_v_map: BTreeMap<Value, Value> = BTreeMap::new();").expect("failed to write Op::Const");
    //                                         for (v_v_k, v_v_v) in v_v.as_array().map.iter() {
    //                                             writeln!(&mut writer, "v_v_map.insert(Field(FieldV::new({}, m_arc.clone())), Field(FieldV::new(Integer::from_str_radix(\"{}\", 10).unwrap(), m_arc.clone())));", v_v_k.as_pf().i(), v_v_v.as_pf().i()).expect("failed to write Op::Const")
    //                                         }
    //                                         writeln!(&mut writer, "v_map.insert(Field(FieldV::new({}, m_arc.clone())), Array(Arr::new(Sort::Field(field_t.clone()), Box::new(Field(FieldV::new(0, m_arc.clone()))), v_v_map, {})));", v_k.as_pf().i(), v_v.as_array().size).expect("failed to write Op::Const");
    //                                     }
    //                                     writeln!(&mut writer, "// default size: {}", v.as_array().default.as_array().size);
    //                                     writeln!(&mut writer, "map.insert(Field(FieldV::new({}, m_arc.clone())), Array(Arr::new(
    //                                             Sort::Field(field_t.clone()),
    //                                             Box::new(Array(Arr::new(Sort::Field(field_t.clone()), Box::new(Field(FieldV::new(0, m_arc.clone()))), Default::default(), {}))),
    //                                             v_map,
    //                                             {}
    //                                         )));", k.as_pf().i(), v.as_array().size, v.as_array().size).expect("failed to write Op::Const");
    //                                 }
    //                                 writeln!(&mut writer, "let id_30 = Array(Arr::new(Sort::Field(field_t.clone()), Box::new(Array(Arr::new(Sort::Field(field_t.clone()), Box::new(Field(FieldV::new(0, m_arc.clone()))), Default::default(), 9))), map, {}))", a.size).expect("failed to write Op::Const");
    //
    //
    //
    //
    //                                 // // println!("let key_sort = Sort::Field(field_t.clone());");
    //                                 // // println!("let default = Box::new(Array(Arr::new(Sort::Field(field_t.clone()), Box::new(Field(FieldV::new(0, m_arc.clone()))), Default::default(), 9)));");
    //                                 // // println!("let mut map: BTreeMap<Value, Value> = BTreeMap::new();");
    //                                 // // println!("{:?}", a.map);
    //                                 // // println!("let mut arr_map: BTreeMap<Value, Value> = BTreeMap::new();");
    //                                 // for (k, v) in a.map.iter() {
    //                                 //     println!("{:?}: key_sort: {:?}, default: {:?}, size: {}", k, v.as_array().key_sort.as_pf(), v.as_array().default.as_array(), v.as_array().size);
    //                                 //     // println!("arr_map.clear();");
    //                                 //     for (v_k, v_v) in v.as_array().map.iter() {
    //                                 //         println!("{:?}:", v_k);
    //                                 //         // println!("let mut v_v_arr_map: BTreeMap<Value, Value> = BTreeMap::new();");
    //                                 //         for (v_v_k, v_v_v) in v_v.as_array().map.iter() {
    //                                 //             println!("v_v_arr_map.insert(Field(FieldV::new({}, m_arc.clone())), Field(FieldV::new(Integer::from_str_radix(\"{}\", 10).unwrap(), m_arc.clone())));", v_v_k.as_pf().i(), v_v_v.as_pf().i());
    //                                 //         }
    //                                 //         // println!("arr_map.insert(Field(FieldV::new({}, m_arc.clone())), Array(Arr::new(Sort::Field(field_t.clone()), Box::new(Array(Arr::new(Sort::Field(field_t.clone()), Box::new(Field(FieldV::new(0, m_arc.clone()))), Default::default(), {}))), v_v_arr_map, {}))", v_v.as_array().size, v_v.as_array().size);
    //                                 //     }
    //                                 //     println!("========")
    //                                 // }
    //                                 // println!("size: {:?}", a.size);
    //                             }
    //                             // writeln!(&mut writer, "let id_{} array sort = {:?}, default: {:?}, map: {:?}, size: {};", id, a.key_sort, a.default.sort(), a.map, a.size);
    //                         }
    //                         Value::Map(m) => {writeln!(&mut writer, "let id_{} map = {:?};", id, m);}
    //                         Value::Tuple(t) => {writeln!(&mut writer, "let id_{} tuple = {};", id, t.len());}
    //                     }
    //
    //                 }
    //                 Op::BvBinOp(o) => {
    //                     match o {
    //                         BvBinOp::Sub => {
    //                             writeln!(&mut writer, "let id_{} = Value::BitVector(id_{}.as_bv().clone() - id_{}.as_bv().clone());", id, operand_ids[0], operand_ids[1]);
    //                         }
    //                         BvBinOp::Udiv => {}
    //                         BvBinOp::Urem => {}
    //                         BvBinOp::Shl => {}
    //                         BvBinOp::Ashr => {}
    //                         BvBinOp::Lshr => {}
    //                     }
    //                 }
    //                 Op::BvNaryOp(o) => {
    //                     // println!("id: {}, operator: {}, value: {}, operands_size: {}", id, operator, value, operand_ids.len());
    //                     match o {
    //                         BvNaryOp::Add => {
    //                             writeln!(&mut writer, "let id_{} = Value::BitVector(id_{}.as_bv().clone().add(id_{}.as_bv().clone()));", id, operand_ids[0], operand_ids[1]);
    //                         }
    //                         BvNaryOp::Mul => {
    //                             writeln!(&mut writer, "let id_{} = Value::BitVector(id_{}.as_bv().clone().mul(id_{}.as_bv().clone()));", id, operand_ids[0], operand_ids[1]);
    //                         }
    //                         BvNaryOp::Or => {}
    //                         BvNaryOp::And => {}
    //                         BvNaryOp::Xor => {}
    //                     }
    //                 }
    //                 Op::Ite => {
    //                     // println!("let id_{} = if id_{}.as_bool() {{ id_{} }} else {{ id_{} }};", id, operand_ids[0], operand_ids[1], operand_ids[2]);
    //                 }
    //                 Op::BvBinPred(o) => {
    //                     match o {
    //                         BvBinPred::Ult => { writeln!(&mut writer, "let id_{} = Value::Bool(id_{}.as_bv().uint() >= id_{}.as_bv().uint());", id, operand_ids[0], operand_ids[1]).expect("failed to write Op::BvBinPred"); }
    //                         BvBinPred::Ugt => { writeln!(&mut writer, "let id_{} = Value::Bool(id_{}.as_bv().uint() > id_{}.as_bv().uint());", id, operand_ids[0], operand_ids[1]).expect("failed to write Op::BvBinPred"); }
    //                         BvBinPred::Ule => { writeln!(&mut writer, "let id_{} = Value::Bool(id_{}.as_bv().uint() <= id_{}.as_bv().uint());", id, operand_ids[0], operand_ids[1]).expect("failed to write Op::BvBinPred"); }
    //                         BvBinPred::Uge => { writeln!(&mut writer, "let id_{} = Value::Bool(id_{}.as_bv().uint() < id_{}.as_bv().uint());", id, operand_ids[0], operand_ids[1]).expect("failed to write Op::BvBinPred"); }
    //                         BvBinPred::Slt => {}
    //                         BvBinPred::Sgt => {}
    //                         BvBinPred::Sle => {}
    //                         BvBinPred::Sge => {}
    //                     }
    //                 }
    //                 Op::BoolToBv | Op::PfUnOp(_) | Op::PfDiv => {
    //                     println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                 }
    //                 Op::PfNaryOp(o) => {
    //                     // println!("id: {}, operator: {}, value: {}, operands_size: {}", id, operator, value, operand_ids.len());
    //                     match o {
    //                         PfNaryOp::Add => {
    //                             writeln!(&mut writer, "let id_{} = id_{}.as_pf().clone().add(id_{}.as_pf().clone());", id, operand_ids[0], operand_ids[1]).expect("failed to write Op::PfNaryOp");
    //                         }
    //                         PfNaryOp::Mul => {
    //                             writeln!(&mut writer, "let id_{} = id_{}.as_pf().clone().mul(id_{}.as_pf().clone());", id, operand_ids[0], operand_ids[1]).expect("failed to write Op::PfNaryOp");
    //                         }
    //                     }
    //                 }
    //                 Op::IntBinPred(_) | Op::IntNaryOp(_) => {
    //                     println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                 }
    //                 Op::UbvToPf(ft) => {
    //                     // TODO: need to define ft
    //                     println!("ft: {}", ft);
    //                     writeln!(&mut writer, "let id_{} = Value::Field(field_t.new_v(id_{}.as_bv().uint()));", id, operand_ids[0]).expect("failed to write Op::UbvToPf");
    //                 }
    //                 Op::PfChallenge(_, _) | Op::Witness(_) | Op::PfFitsInBits(_) => {
    //                     println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                 }
    //                 Op::Tuple => {
    //                     // Value::Tuple(args.iter().map(|a| (*a).clone()).collect())
    //                     write!(&mut writer, "let mut operands: [Value; {}] = [", operand_ids.len()).expect("failed to write Op::Tuple");
    //                     for operand_id in operand_ids {
    //                         write!(&mut writer, "id_{}.clone(),", operand_id).expect("failed to write Op::Tuple");
    //                     }
    //                     writeln!(&mut writer, "];").expect("failed to write Op::Tuple");
    //                     writeln!(&mut writer, "let id_{} = Value::Tuple(Box::new(operands));", id).expect("failed to write Op::Tuple");
    //                 }
    //                 Op::Field(i) => {
    //                     writeln!(&mut writer, "let mut field_i = {};", i).expect("failed to write Op::Field");
    //                     writeln!(&mut writer, "let id_{} = id_{}.as_tuple()[field_i].clone();", id, operand_ids[0]).expect("failed to write Op::Field");
    //                 }
    //                 Op::Update(i) => {
    //                     writeln!(&mut writer, "let mut update_i = {};", i).expect("failed to write Op::Update");
    //                     writeln!(&mut writer, "let mut t = Vec::from(id_{}.as_tuple()).into_boxed_slice();", operand_ids[0]).expect("failed to write Op::Update");
    //                     writeln!(&mut writer, "t[update_i] = id_{}.clone();", operand_ids[1]).expect("failed to write Op::Update");
    //                     writeln!(&mut writer, "let id_{} = Value::Tuple(t);", id).expect("failed to write Op::Update");
    //                 }
    //                 Op::CStore => {
    //                     println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                 }
    //                 Op::Store => {
    //                     writeln!(&mut writer, "let id_{} = Value::Array(id_{}.as_array().clone().store(id_{}.clone(), id_{}.clone()));", id, operand_ids[0], operand_ids[1], operand_ids[2]).expect("failed to write Op::Store");
    //                 }
    //                 Op::Array(_, _) => {
    //                     println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                 }
    //                 Op::Select => {
    //                     writeln!(&mut writer, "let id_{} = id_{}.as_array().select(&id_{});", id, operand_ids[0], operand_ids[1]).expect("failed to write Op::Select");
    //                 }
    //                 Op::Map(_) | Op::Rot(_) | Op::PfToBoolTrusted | Op::ExtOp(_) => {
    //                     println!("id: {}, operator: {}", id, operator);
    //                 },
    //                 Op::Fill(key_sort, size) => {
    //                     println!("Sort: {}", key_sort.as_pf());
    //                     writeln!(&mut writer, "let id_{} = Value::Array(Array::new(Sort::Field(field_t), Box::new(id_{}.clone()), Default::default(), {}));", id, operand_ids[0], size.clone()).expect("failed to write Op::Fill");
    //                 }
    //                 _ => {
    //                     println!("id: {}, operator: {}, value: {}", id, operator, value);
    //                 }
    //             }
    //         }
    //
    //         self.step_values.push(value.clone());
    //     }
    //     writeln!(&mut writer, "}}").expect("failed to write a function def");
    // }

    // pub fn eval_stages(&mut self, eval_operations: &BTreeMap<usize, (Op, Vec<usize>)>, inputs: &mut HashMap<String, Value>) -> Vec<&Value> {
    //     println!("========== EVAL_STAGE ==========");
    //     let total_timer = Instant::now();
    //
    //     let mut out = Vec::new();
    //     self.variable_values.extend(std::mem::take(inputs));
    //     &self.eval_step_with(&eval_operations);
    //
    //     for stage in &self.comp.stages {
    //         let num_outputs = stage.num_outputs;
    //         // self.variable_values.extend(std::mem::take(inputs));
    //         // if num_outputs > 0 {
    //         //     &self.eval_step_with(&eval_operations);
    //         // }
    //         self.outputs_evaluted += num_outputs;
    //         self.stages_evaluated += 1;
    //     }
    //
    //     for output_step in
    //         &self.comp.ouput_steps[0..self.outputs_evaluted]
    //     {
    //         out.push(&self.step_values[*output_step]);
    //     }
    //
    //     // Self::write_eval_operations(&eval_step_operations).expect("write eval operations failed");
    //
    //     // println!("eval_steps.size: {}", eval_step_operations.len());
    //     println!("eval_stage elapsed: {:.2?}", total_timer.elapsed());
    //     out
    // }

    /// Evaluate one stage.
    pub fn eval_stage(&mut self, inputs: HashMap<String, Value>) -> Vec<&Value> {
        let eval_operations = Self::read_eval_operations().unwrap();

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

        // let mut eval_step_operations: BTreeMap<usize, (Op, Vec<usize>)> = BTreeMap::default();

        if num_outputs > 0 {
            let max_step = (0..num_outputs)
                .map(|i| {
                    let new_output_i = i + self.outputs_evaluted;
                    self.comp.ouput_steps[new_output_i]
                })
                .max()
                .unwrap();
            while self.step_values.len() <= max_step {
                self.eval_step()

                // getting operation (optimization)
                // let (id, operation) = self.eval_step_operation();
                // eval_step_operations.insert(id, operation);
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

        // Self::write_eval_operations(&eval_step_operations).expect("write eval operations failed");

        // println!("eval_steps.size: {}", eval_step_operations.len());
        println!("eval_stage elapsed: {:.2?}", total_timer.elapsed());
        out
    }

    /// Prints out operator evaluation times (if self.time_ops is set)
    pub fn print_times(&self) {
        if self.time_ops {
            // (operator, nanos total, counts, nanos/count, arg sorts (or *))
            let mut rows: Vec<(String, usize, usize, f64, String)> = Default::default();
            for ((op, arg_sorts), (time, count)) in &self.op_times {
                let nanos = time.as_nanos() as usize;
                let per = nanos as f64 / *count as f64;
                rows.push((
                    format!("{}", op),
                    nanos,
                    *count,
                    per,
                    format!("{:?}", arg_sorts),
                ));
            }
            rows.sort_by_key(|t| t.1);
            println!("time,op,nanos,counts,nanos_per,arg_sorts");
            for (op, nanos, counts, nanos_per, arg_sorts) in &rows {
                println!("time,{op},{nanos},{counts},{nanos_per},\"{arg_sorts}\"");
            }
        }
    }

    fn write_eval_operations(eval_operations: &BTreeMap<usize, (Op, Vec<usize>)>) -> io::Result<()> {
        let file_path = if Path::new("EVAL.json").exists() { Path::new("EVAL_1.json") } else { Path::new("EVAL.json") };
        let mut file = BufWriter::new(File::create(file_path).unwrap());
        bincode::serde::encode_into_std_write(&eval_operations, &mut file, bincode::config::legacy()).unwrap();
        Ok(())
    }

    pub fn read_eval_operations() -> io::Result<BTreeMap<usize, (Op, Vec<usize>)>> {
        let mut total_operations: BTreeMap<usize, (Op, Vec<usize>)> = BTreeMap::default();

        let mut file = BufReader::new(File::open("EVAL.json")?);
        let mut operations: BTreeMap<usize, (Op, Vec<usize>)> = bincode::serde::decode_from_std_read(&mut file, bincode::config::legacy()).unwrap();
        total_operations.extend(operations);

        file = BufReader::new(File::open("EVAL_1.json")?);
        operations = bincode::serde::decode_from_std_read(&mut file, bincode::config::legacy()).unwrap();
        total_operations.extend(operations);

        Ok(total_operations)
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
            var("b".into(), Sort::Field(field.clone())),
            term![Op::Ite; var("a".into(), Sort::Bool), pf_lit(field.new_v(1)), pf_lit(field.new_v(0))],
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
            var("b".into(), Sort::Field(field.clone())),
            term![Op::Ite; var("a".into(), Sort::Bool), pf_lit(field.new_v(1)), pf_lit(field.new_v(0))],
        ]);
        comp.add_stage(mk_inputs(vec![("c".into(), Sort::Field(field.clone()))]),
        vec![
            term![PF_ADD;
               var("b".into(), Sort::Field(field.clone())),
               var("c".into(), Sort::Field(field.clone()))],
            term![Op::Ite; var("a".into(), Sort::Bool), pf_lit(field.new_v(1)), pf_lit(field.new_v(0))],
            term![Op::Ite; var("a".into(), Sort::Bool), pf_lit(field.new_v(0)), pf_lit(field.new_v(1))],
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
