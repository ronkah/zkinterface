use flatbuffers::{FlatBufferBuilder, WIPOffset};
use serde::{Deserialize, Serialize};
use crate::zkinterface_generated::zkinterface::{Gate, GateArgs, GateSet, GateConstant, GateConstantArgs, Wire, GateAssertZero, GateAdd, GateMul, GateAssertZeroArgs, GateAddArgs, GateMulArgs, GateInstanceVar, GateInstanceVarArgs, GateWitness, GateWitnessArgs};

type WireId = u64;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum GateOwned {
    Constant(WireId, Vec<u8>),
    InstanceVar(WireId),
    Witness(WireId),
    AssertZero(WireId),
    Add(WireId, WireId, WireId),
    Mul(WireId, WireId, WireId),
}

use GateOwned::*;


impl<'a> From<Gate<'a>> for GateOwned {
    /// Convert from Flatbuffers references to owned structure.
    fn from(gate_ref: Gate) -> GateOwned {
        match gate_ref.gate_type() {
            GateSet::GateConstant => {
                let gate = gate_ref.gate_as_gate_constant().unwrap();
                GateOwned::Constant(
                    gate.output().unwrap().id(),
                    Vec::from(gate.constant().unwrap()))
            }

            GateSet::GateInstanceVar => {
                let gate = gate_ref.gate_as_gate_instance_var().unwrap();
                GateOwned::InstanceVar(
                    gate.output().unwrap().id())
            }

            GateSet::GateWitness => {
                let gate = gate_ref.gate_as_gate_witness().unwrap();
                GateOwned::Witness(
                    gate.output().unwrap().id())
            }

            GateSet::GateAssertZero => {
                let gate = gate_ref.gate_as_gate_assert_zero().unwrap();
                GateOwned::AssertZero(
                    gate.input().unwrap().id())
            }

            GateSet::GateAdd => {
                let gate = gate_ref.gate_as_gate_add().unwrap();
                GateOwned::Add(
                    gate.output().unwrap().id(),
                    gate.left().unwrap().id(),
                    gate.right().unwrap().id())
            }

            GateSet::GateMul => {
                let gate = gate_ref.gate_as_gate_mul().unwrap();
                GateOwned::Mul(
                    gate.output().unwrap().id(),
                    gate.left().unwrap().id(),
                    gate.right().unwrap().id())
            }

            _ => unimplemented!()
        }
    }
}

impl GateOwned {
    pub fn has_output(&self) -> bool {
        match *self {
            AssertZero(_) => false,
            _ => true,
        }
    }

    pub fn get_output(&self) -> WireId {
        match *self {
            Constant(o, _) => o,
            InstanceVar(o) => o,
            Witness(o) => o,
            AssertZero(_) => 0,
            Add(o, _, _) => o,
            Mul(o, _, _) => o,
        }
    }

    pub fn with_output(self, o: WireId) -> Self {
        match self {
            Constant(_, c) => Constant(o, c),
            InstanceVar(_) => InstanceVar(o),
            Witness(_) => Witness(o),
            AssertZero(_) => self,
            Add(_, l, r) => Add(o, l, r),
            Mul(_, l, r) => Mul(o, l, r),
        }
    }

    pub fn cacheable(&self) -> bool {
        match *self {
            InstanceVar(_) | Witness(_) => false,
            Constant(_, _) | AssertZero(_) | Add(_, _, _) | Mul(_, _, _) => true,
        }
    }

    /// Add this structure into a Flatbuffers message builder.
    pub fn build<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr>(
        &'args self,
        builder: &'mut_bldr mut FlatBufferBuilder<'bldr>,
    ) -> WIPOffset<Gate<'bldr>>
    {
        match self {
            GateOwned::Constant(output, constant) => {
                let cons = builder.create_vector(constant);
                let gate = GateConstant::create(builder, &GateConstantArgs {
                    output: Some(&Wire::new(*output)),
                    constant: Some(cons),
                });
                Gate::create(builder, &GateArgs {
                    gate_type: GateSet::GateConstant,
                    gate: Some(gate.as_union_value()),
                })
            }

            GateOwned::InstanceVar(output) => {
                let gate = GateInstanceVar::create(builder, &GateInstanceVarArgs {
                    output: Some(&Wire::new(*output)),
                });
                Gate::create(builder, &GateArgs {
                    gate_type: GateSet::GateInstanceVar,
                    gate: Some(gate.as_union_value()),
                })
            }

            GateOwned::Witness(output) => {
                let gate = GateWitness::create(builder, &GateWitnessArgs {
                    output: Some(&Wire::new(*output)),
                });
                Gate::create(builder, &GateArgs {
                    gate_type: GateSet::GateWitness,
                    gate: Some(gate.as_union_value()),
                })
            }

            GateOwned::AssertZero(input) => {
                let gate = GateAssertZero::create(builder, &GateAssertZeroArgs {
                    input: Some(&Wire::new(*input)),
                });
                Gate::create(builder, &GateArgs {
                    gate_type: GateSet::GateAssertZero,
                    gate: Some(gate.as_union_value()),
                })
            }

            GateOwned::Add(output, left, right) => {
                let gate = GateAdd::create(builder, &GateAddArgs {
                    output: Some(&Wire::new(*output)),
                    left: Some(&Wire::new(*left)),
                    right: Some(&Wire::new(*right)),
                });
                Gate::create(builder, &GateArgs {
                    gate_type: GateSet::GateAdd,
                    gate: Some(gate.as_union_value()),
                })
            }

            GateOwned::Mul(output, left, right) => {
                let gate = GateMul::create(builder, &GateMulArgs {
                    output: Some(&Wire::new(*output)),
                    left: Some(&Wire::new(*left)),
                    right: Some(&Wire::new(*right)),
                });
                Gate::create(builder, &GateArgs {
                    gate_type: GateSet::GateMul,
                    gate: Some(gate.as_union_value()),
                })
            }
        }
    }
}
