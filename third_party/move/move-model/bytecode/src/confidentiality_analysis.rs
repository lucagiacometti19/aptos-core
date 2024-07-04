// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This confidentiality analysis flags explicit and inplicit flow of data
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashSet},
};

use codespan::FileId;
use codespan_reporting::diagnostic::{Diagnostic, Label, Severity};

use move_binary_format::file_format::CodeOffset;
use move_model::{
    ast::{Operation as ASTOperation, TempIndex},
    model::{FieldId, FunctionEnv, ModuleId, QualifiedId, StructId},
};

use crate::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, CustomState, JoinResult},
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Operation},
    stackless_control_flow_graph::{generate_cfg_in_dot_format, StacklessControlFlowGraph},
};

// =================================================================================================
// Data Model

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AbsValue {
    P,
    S,
}

impl AbsValue {
    pub fn is_secret(&self) -> bool {
        matches!(self, Self::S)
    }

    pub fn least_upper_bound(self, x: &AbsValue) -> AbsValue {
        match (&self, &x) {
            (AbsValue::P, AbsValue::P) => AbsValue::P,
            _ => AbsValue::S,
        }
    }
}

type ConfidentialityAnalysisState = CustomState<TempIndex, AbsValue>;

impl ConfidentialityAnalysisState {
    fn get_pc_value(&self) -> &AbsValue {
        self.get_stack()
            .back()
            .unwrap_or_else(|| panic!("Unbound pc in state {:?}", self))
    }

    fn push_pc_value(&mut self, value: AbsValue) {
        self.get_stack_mut().push_back(value);
    }

    fn get_local_index(&self, i: &TempIndex) -> &AbsValue {
        self.get_map()
            .get(&i)
            .unwrap_or_else(|| panic!("Unbound local in state {:?}", self))
    }

    fn assign(&mut self, lhs: TempIndex, rhs: &TempIndex) {
        let to_propagate = self
            .get_local_index(rhs)
            .least_upper_bound(self.get_pc_value());
        self.get_map_mut().insert(lhs, to_propagate);
    }

    fn add_local(&mut self, i: TempIndex, value: AbsValue) {
        self.get_map_mut().insert(i, value);
    }

    /// Add locals to state with value
    fn call(&mut self, locals: &Vec<TempIndex>, value: AbsValue) {
        for loc_index in locals {
            self.add_local(*loc_index, value);
        }
    }
}

// =================================================================================================
// Joins

impl AbstractDomain for AbsValue {
    fn join(&mut self, other: &Self) -> JoinResult {
        if self == other {
            return JoinResult::Unchanged;
        }
        // unequal; use top value
        *self = AbsValue::S;
        JoinResult::Changed
    }
}

// =================================================================================================
// Transfer functions

#[derive(PartialOrd, PartialEq, Eq, Ord)]
struct WarningId {
    index: usize,
    offset: CodeOffset,
}

#[derive(Debug)]
struct SpecMemoryInfo {
    /// Fields that occur in struct, module, or global specs. Leaked references to fields inside
    /// this set will be flagged, leaked references to other fields will be allowed.
    relevant_fields: BTreeSet<(QualifiedId<StructId>, FieldId)>,
    /// Structs that occur in struct, module, or global specs. Leaked references to fields inside
    /// these structs may cause a spec like `invariant forall s: S: s == S { f: 10 }` to be false
    relevant_structs: BTreeSet<QualifiedId<StructId>>,
    /// Vector-related operations that occur in struct, module, or global specs. Leaked references
    /// to vector contents will be allowed if this is empty
    vector_operations: HashSet<ASTOperation>,
}

struct ConfidentialityAnalysis<'a> {
    func_env: &'a FunctionEnv<'a>,
    /// Warnings about data flows to surface to the programmer
    // Uses a map instead of a vec to avoid reporting multiple warnings
    // at program locations in a loop during fixpoint iteration
    leak_warnings: RefCell<BTreeMap<WarningId, Diagnostic<FileId>>>,
    /// Information about the memory touched by the specs of the declaring module for this function
    /// If the function's declaring module has no specs, this will be None
    spec_memory: Option<SpecMemoryInfo>,
}

impl ConfidentialityAnalysis<'_> {
    pub fn add_leaking_call_warning(
        &self,
        call_index: usize,
        is_explicit: bool,
        offset: CodeOffset,
    ) {
        let fun_loc = self.func_env.get_loc();
        let binding = self.func_env.get_local_name(call_index);
        let local_name = binding.display(self.func_env.symbol_pool());
        let message = if is_explicit {
            format!("Explicit data leak via call with local {}", local_name)
        } else {
            format!("Implicit data leak via call")
        };
        let label = Label::primary(fun_loc.file_id(), fun_loc.span());
        let severity = Severity::Warning;
        let warning_id = WarningId {
            index: call_index,
            offset,
        };
        self.leak_warnings.borrow_mut().insert(
            warning_id,
            Diagnostic::new(severity)
                .with_message(message)
                .with_labels(vec![label]),
        );
    }

    pub fn add_leaking_return_warning(
        &self,
        ret_index: usize,
        is_explicit: bool,
        offset: CodeOffset,
    ) {
        let fun_loc = self.func_env.get_loc();
        let binding = self.func_env.get_local_name(ret_index);
        let local_name = binding.display(self.func_env.symbol_pool());
        let message = if is_explicit {
            format!("Explicit data leak via return of local {}", local_name)
        } else {
            format!("Implicit data leak via return - off: {}", offset)
        };
        let label = Label::primary(fun_loc.file_id(), fun_loc.span());
        let severity = Severity::Warning;
        let warning_id = WarningId {
            index: ret_index,
            offset,
        };
        self.leak_warnings.borrow_mut().insert(
            warning_id,
            Diagnostic::new(severity)
                .with_message(message)
                .with_labels(vec![label]),
        );
    }

    pub fn add_leaking_move_warning(&self, move_index: usize, offset: CodeOffset) {
        let fun_loc = self.func_env.get_loc();
        let binding = self.func_env.get_local_name(move_index);
        let local_name = binding.display(self.func_env.symbol_pool());
        let message = format!("Data leak via moveTo of local {}", local_name);
        let label = Label::primary(fun_loc.file_id(), fun_loc.span());
        let severity = Severity::Warning;
        let warning_id = WarningId {
            index: move_index,
            offset,
        };
        self.leak_warnings.borrow_mut().insert(
            warning_id,
            Diagnostic::new(severity)
                .with_message(message)
                .with_labels(vec![label]),
        );
    }

    pub fn add_leaking_writeref_warning(&self, write_index: usize, offset: CodeOffset) {
        let fun_loc = self.func_env.get_loc();
        let binding = self.func_env.get_local_name(write_index);
        let local_name = binding.display(self.func_env.symbol_pool());
        let message = format!("Data leak via write ref of local {}", local_name);
        let label = Label::primary(fun_loc.file_id(), fun_loc.span());
        let severity = Severity::Warning;
        let warning_id = WarningId {
            index: write_index,
            offset,
        };
        self.leak_warnings.borrow_mut().insert(
            warning_id,
            Diagnostic::new(severity)
                .with_message(message)
                .with_labels(vec![label]),
        );
    }

    /// TODO: 'we consider all structs to be relevant in that case' correct?
    /// Return true if `sid` is mentioned in a specification of the current module *or* if the
    /// module has no specifications (i.e., we consider all structs to be relevant in that case)
    pub fn specs_contain_struct(&self, mid: &ModuleId, sid: &StructId) -> bool {
        // TODO: debug only
        //println!("specs: {:#?}", &self.spec_memory);
        if let Some(specs) = &self.spec_memory {
            // TODO: debug only
            //println!("specs: {:#?}", specs);
            let qsid = mid.qualified(*sid);
            specs.relevant_structs.contains(&qsid)
        } else {
            true
        }
    }

    /// Return true if `fld` is mentioned in a specification of the current module *or* if the
    /// module has no specifications (i.e., we consider all fields to be relevant in that case)
    pub fn specs_contain_field(&self, mid: &ModuleId, sid: &StructId, fld: &FieldId) -> bool {
        if let Some(specs) = &self.spec_memory {
            let qsid = mid.qualified(*sid);
            specs.relevant_structs.contains(&qsid) || specs.relevant_fields.contains(&(qsid, *fld))
        } else {
            true
        }
    }

    /// Return `true` if vector indexes are mentioned in a specification of the current module *or*
    /// if the module has no specifications
    pub fn specs_contain_vector_index(&self) -> bool {
        use ASTOperation::*;
        if let Some(specs) = &self.spec_memory {
            for op in &specs.vector_operations {
                match op {
                    // TODO: not sure about SingleVec, IndexOf, ContainsVec, InRangeVec, RangeVec
                    Index | Slice | UpdateVec | SingleVec | IndexOfVec | ContainsVec
                    | InRangeVec | RangeVec => return true,
                    _ => (),
                }
            }
            false
        } else {
            true
        }
    }

    /// Returns `true` if vector lengths are mentioned in a specification of the current module *or*
    /// if the module has no specifications
    pub fn specs_contain_vector_length(&self) -> bool {
        use ASTOperation::*;
        if let Some(specs) = &self.spec_memory {
            for op in &specs.vector_operations {
                match op {
                    // TODO: does every indexing-related operation belong here?
                    Len | SingleVec | EmptyVec => return true,
                    _ => (),
                }
            }
            false
        } else {
            true
        }
    }
}

impl<'a> TransferFunctions for ConfidentialityAnalysis<'a> {
    type State = ConfidentialityAnalysisState;
    //false -> forward data flow analysis | true -> backward data flow analysis
    const BACKWARD: bool = false;

    fn execute(&self, state: &mut Self::State, instr: &Bytecode, offset: CodeOffset) {
        use Bytecode::*;
        use Operation::*;

        match instr {
            Call(_, rets, oper, args, _) => {
                // map secret args
                let secret_args: Vec<&TempIndex> = args
                    .iter()
                    .filter(|&arg_index| state.get_local_index(arg_index).is_secret())
                    .collect();

                /* state.call(
                    rets,
                    if secret_args.is_empty() {
                        AbsValue::P
                    } else {
                        AbsValue::S
                    },
                ); */
                match oper {
                    Function(mid, fid, _) => {
                        let env = self.func_env.module_env.env;
                        let callee_fun_env = env.get_function(mid.qualified(*fid));
                        //let addr = callee_fun_env.module_env.get_name().addr();
                        //let stdlib_add = &env.get_stdlib_address();
                        //////
                        let binding = callee_fun_env.get_name();
                        let fn_name = binding.display(callee_fun_env.symbol_pool());
                        println!("Name: {} | Current instr: {:?}", fn_name, instr);
                        println!("Current state: {:?}", state.get_map_mut());
                        ////
                        if callee_fun_env.is_native()
                        /* || addr == stdlib_add */
                        {
                            ////
                            println!("NATIVE");
                            ////
                            // todo: currently native fn & stdlib fn do not cause diags
                            match callee_fun_env.module_env.get_identifier().unwrap().as_str() {
                                "vector" => {
                                    match callee_fun_env.get_identifier().unwrap().as_str() {
                                        "borrow_mut" | "borrow" => {
                                            let vec_arg = 0;
                                            let to_propagate =
                                                match state.get_local_index(&args[vec_arg]) {
                                                    AbsValue::P => {
                                                        if self.specs_contain_vector_index() {
                                                            AbsValue::S
                                                        } else {
                                                            AbsValue::P
                                                        }
                                                    },
                                                    AbsValue::S => AbsValue::S,
                                                };
                                            state.add_local(rets[0], to_propagate);
                                        },
                                        //"push_back" => {
                                        //
                                        //},
                                        _ => {
                                            state.call(
                                                rets,
                                                if secret_args.is_empty() {
                                                    AbsValue::P
                                                } else {
                                                    AbsValue::S
                                                },
                                            );
                                        },
                                    }
                                },
                                _ => {
                                    state.call(
                                        rets,
                                        if secret_args.is_empty() {
                                            AbsValue::P
                                        } else {
                                            AbsValue::S
                                        },
                                    );
                                },
                            }
                        } else {
                            state.call(
                                rets,
                                if secret_args.is_empty() {
                                    AbsValue::P
                                } else {
                                    AbsValue::S
                                },
                            );

                            // flag call during secret pc state - implicit flow
                            if state.get_pc_value().is_secret() {
                                self.add_leaking_call_warning(0, false, offset);
                            }
                            // check for explicit flow via call args - flag secret args
                            for secret_arg in secret_args {
                                self.add_leaking_call_warning(*secret_arg, true, offset);
                            }
                        }
                    },
                    BorrowLoc => {
                        state.add_local(rets[0], *state.get_local_index(&args[0]));
                    },
                    BorrowField(mid, sid, _type_params, offset) => {
                        let struct_env = self.func_env.module_env.get_struct(*sid);
                        let field_env = struct_env.get_field_by_offset(*offset);
                        let field_id = field_env.get_id();
                        // todo: specs_contain_field and specs_contain_vector_length are problems if there are no specs
                        // line 234 and 245 to false?
                        let to_propagate = if self.specs_contain_field(mid, sid, &field_id)
                            || (field_env.get_type().is_vector()
                                && self.specs_contain_vector_length())
                        {
                            AbsValue::S
                        } else {
                            AbsValue::P
                        };
                        state.add_local(
                            rets[0],
                            to_propagate.least_upper_bound(state.get_local_index(&args[0])),
                        );
                    },
                    ReadRef => {
                        state.add_local(
                            rets[0],
                            state
                                .get_local_index(&args[0])
                                .least_upper_bound(state.get_pc_value()),
                        );
                    },
                    BorrowGlobal(mid, sid, _type_params) => {
                        let to_propagate = if self.specs_contain_struct(mid, sid) {
                            AbsValue::S
                        } else {
                            AbsValue::P
                        };
                        state.add_local(
                            rets[0],
                            to_propagate.least_upper_bound(state.get_local_index(&args[0])),
                        );
                    },
                    Pack(..) => {
                        // LUP of args
                        let to_propagate = if args
                            .iter()
                            .any(|arg_index| state.get_local_index(arg_index).is_secret())
                        {
                            AbsValue::S
                        } else {
                            AbsValue::P
                        };
                        state.add_local(rets[0], to_propagate);
                    },
                    MoveFrom(mid, sid, _type_params) => {
                        let to_propagate = if self.specs_contain_struct(mid, sid) {
                            AbsValue::S
                        } else {
                            AbsValue::P
                        };
                        state.add_local(
                            rets[0],
                            to_propagate.least_upper_bound(state.get_pc_value()),
                        );
                    },
                    CastU8 | CastU16 | CastU32 | CastU64 | CastU128 | CastU256 | Not => {
                        // unary ops
                        state.add_local(rets[0], *state.get_local_index(&args[0]));
                    },
                    Add | Sub | Mul | Div | Mod | BitOr | BitAnd | Xor | Shl | Shr | Lt | Gt
                    | Le | Ge | Or | And | Eq | Neq => {
                        // binary ops
                        state.add_local(
                            rets[0],
                            state
                                .get_local_index(&args[0])
                                .least_upper_bound(state.get_local_index(&args[1])),
                        );
                    },
                    Unpack(..) => {
                        rets.iter().for_each(|&ret_index| {
                            state.add_local(ret_index, *state.get_local_index(&args[0]));
                        });
                    },
                    WriteRef => {
                        state.call(
                            rets,
                            if secret_args.is_empty() {
                                AbsValue::P
                            } else {
                                AbsValue::S
                            },
                        );

                        if *state.get_local_index(&args[1]) == AbsValue::P {
                            // allowed only if pc is P
                            if state.get_pc_value().is_secret() {
                                self.add_leaking_writeref_warning(args[1], offset)
                            }
                        }
                    },
                    MoveTo(mid, sid, _) => {
                        state.call(
                            rets,
                            if secret_args.is_empty() {
                                AbsValue::P
                            } else {
                                AbsValue::S
                            },
                        );

                        // struct is mentioned by an inv OR the reference to move is secret
                        if self.specs_contain_struct(mid, sid)
                            || state.get_local_index(&args[0]).is_secret()
                        {
                            // only allowed if pc is secret
                            if *state.get_pc_value() == AbsValue::P {
                                self.add_leaking_move_warning(args[0], offset)
                            }
                        }
                    },
                    Uninit => {
                        state.call(
                            rets,
                            if secret_args.is_empty() {
                                AbsValue::P
                            } else {
                                AbsValue::S
                            },
                        );
                    },
                    Exists(..) => {
                        state.call(
                            rets,
                            if secret_args.is_empty() {
                                AbsValue::P
                            } else {
                                AbsValue::S
                            },
                        );
                    },
                    FreezeRef(..) => {
                        state.call(
                            rets,
                            if secret_args.is_empty() {
                                AbsValue::P
                            } else {
                                AbsValue::S
                            },
                        );
                    },
                    // todo: what are these?
                    Drop => (),
                    Release => (),
                    Vector => (),
                    oper => panic!("unsupported oper {:?}", oper),
                }
            },
            Ret(_, rets) => {
                if !rets.is_empty() {
                    // flag returns during secret pc state - implicit flow
                    if state.get_pc_value().is_secret() {
                        self.add_leaking_return_warning(0, false, offset);
                    }
                    // flag returns of secret locals - explicit flow
                    for ret_index in rets {
                        if state.get_local_index(ret_index).is_secret() {
                            self.add_leaking_return_warning(*ret_index, true, offset);
                        }
                    }
                }
            },
            Branch(_, _, _, guard) => state.push_pc_value(*state.get_local_index(guard)),
            Assign(_, lhs, rhs, _) => {
                state.assign(*lhs, rhs);
            },
            Load(_, lhs, _) => {
                state.add_local(*lhs, *state.get_pc_value());
            },
            Jump(..) | Label(..) | Abort(..) | Nop(..) | SaveMem(..) | SaveSpecVar(..)
            | Prop(..) | SpecBlock(..) => (),
        }
    }
}

impl<'a> DataflowAnalysis for ConfidentialityAnalysis<'a> {}
pub struct ConfidentialityAnalysisProcessor();
impl ConfidentialityAnalysisProcessor {
    pub fn new() -> Box<Self> {
        Box::new(ConfidentialityAnalysisProcessor())
    }
}

impl FunctionTargetProcessor for ConfidentialityAnalysisProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            return data;
        }

        let mut initial_state = ConfidentialityAnalysisState::default();
        // map every local
        for tmp_idx in 0..func_env.get_local_count().unwrap() {
            // if local is a parameter, flag it as secret
            // TODO: need to considerate invariants
            initial_state.add_local(
                tmp_idx,
                if tmp_idx < func_env.get_parameter_count() {
                    AbsValue::S
                } else {
                    AbsValue::P
                },
            );
        }

        // initialize pc value
        initial_state.push_pc_value(AbsValue::P);

        let cfg = StacklessControlFlowGraph::new_forward(&data.code);
        //TODO: debug only - delete
        let func_target = crate::function_target::FunctionTarget::new(&func_env, &data);
        let binding = func_target.get_name();
        let fn_name = binding.display(func_target.symbol_pool());
        println!("{}", fn_name);
        /*  && fn_name.to_string() != "update_price" */
        if fn_name.to_string() != "placeBet" && fn_name.to_string() != "deposit_coinstore" {
            return data;
        }
        let dot_graph = generate_cfg_in_dot_format(&func_target, true);
        println!("{}", dot_graph);

        // compute set of fields and vector ops used in all struct specs
        // Note: global and module specs are not relevant here because
        // it is not possible to leak a reference to a global outside of
        // the module that declares it.
        let mut has_specs = false;
        let menv = &func_env.module_env;
        let mut relevant_fields = BTreeSet::new();
        let mut relevant_structs = BTreeSet::new();
        let mut vector_operations = HashSet::new();
        //for spec_var in menv.get_spec_vars() {
        //    println!("struct_evn: {:#?}", spec_var);
        //}
        //println!("struct_evn: {:#?}", menv.get_spec_vars());
        for struct_env in menv.get_structs() {
            let struct_spec = struct_env.get_spec();
            //println!("struct_spec: {:#?}", struct_spec);
            if !struct_spec.conditions.is_empty() {
                relevant_structs.insert(struct_env.get_qualified_id());
            }
            for condition in &struct_spec.conditions {
                for exp in condition.all_exps() {
                    exp.field_usage(&mut relevant_fields);
                    exp.struct_usage(&mut relevant_structs);
                    exp.vector_usage(&mut vector_operations);
                    has_specs = true
                }
            }
        }
        println!("relevant fields: {:#?}", relevant_fields);
        println!("relevant structs: {:#?}", relevant_structs);

        let analysis = ConfidentialityAnalysis {
            func_env,
            leak_warnings: RefCell::new(BTreeMap::new()),
            spec_memory: if has_specs {
                Some(SpecMemoryInfo {
                    relevant_fields,
                    relevant_structs,
                    vector_operations,
                })
            } else {
                None
            },
        };
        analysis.analyze_function(initial_state, &data.code, &cfg);
        let env = func_env.module_env.env;
        for (_, warning) in analysis.leak_warnings.into_inner() {
            env.add_diag(warning)
        }
        data
    }

    fn name(&self) -> String {
        "confidentiality_analysis".to_string()
    }
}
