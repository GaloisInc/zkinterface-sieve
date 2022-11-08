use crate::Result;
use flatbuffers::{FlatBufferBuilder, ForwardsUOffset, Vector, WIPOffset};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::error::Error;

use super::wire::build_wire_list;
use super::wire::WireList;
use crate::sieve_ir_generated::sieve_ir as generated;
use crate::sieve_ir_generated::sieve_ir::DirectiveSet as ds;
use crate::structs::wire::{expand_wirelist, replace_wire_id};
use crate::{TypeId, Value, WireId};

/// This one correspond to Directive in the FlatBuffers schema
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum Gate {
    /// Constant(type_id, output, constant)
    Constant(TypeId, WireId, Value),
    /// AssertZero(type_id, input)
    AssertZero(TypeId, WireId),
    /// Copy(type_id, output, input)
    Copy(TypeId, WireId, WireId),
    /// Add(type_id, output, input, input)
    Add(TypeId, WireId, WireId, WireId),
    /// Mul(type_id, output, input, input)
    Mul(TypeId, WireId, WireId, WireId),
    /// AddConstant(type_id, output, input, constant)
    AddConstant(TypeId, WireId, WireId, Value),
    /// MulConstant(type_id, output, input, constant)
    MulConstant(TypeId, WireId, WireId, Value),
    /// PublicInput(type_id, output)
    PublicInput(TypeId, WireId),
    /// PrivateInput(type_id, output)
    PrivateInput(TypeId, WireId),
    /// New(type_id, first, last)
    /// Allocate in a contiguous space all wires between the first and the last INCLUSIVE.
    New(TypeId, WireId, WireId),
    /// Delete(type_id, first, last)
    /// All wires between the first and the last INCLUSIVE are deleted.
    /// Last could be equal to first when we would like to delete only one wire
    Delete(TypeId, WireId, WireId),
    /// Convert(output, input)
    Convert(WireList, WireList),
    /// GateCall(name, output_wires, input_wires)
    Call(String, WireList, WireList),
}

use Gate::*;

impl<'a> TryFrom<generated::Directive<'a>> for Gate {
    type Error = Box<dyn Error>;

    /// Convert from Flatbuffers references to owned structure.
    fn try_from(gen_gate: generated::Directive) -> Result<Gate> {
        Ok(match gen_gate.directive_type() {
            ds::NONE => return Err("No gate type".into()),

            ds::GateConstant => {
                let gate = gen_gate.directive_as_gate_constant().unwrap();
                Constant(
                    gate.type_id(),
                    gate.out_id(),
                    Vec::from(gate.constant().ok_or("Missing constant")?),
                )
            }

            ds::GateAssertZero => {
                let gate = gen_gate.directive_as_gate_assert_zero().unwrap();
                AssertZero(gate.type_id(), gate.in_id())
            }

            ds::GateCopy => {
                let gate = gen_gate.directive_as_gate_copy().unwrap();
                Copy(gate.type_id(), gate.out_id(), gate.in_id())
            }

            ds::GateAdd => {
                let gate = gen_gate.directive_as_gate_add().unwrap();
                Add(
                    gate.type_id(),
                    gate.out_id(),
                    gate.left_id(),
                    gate.right_id(),
                )
            }

            ds::GateMul => {
                let gate = gen_gate.directive_as_gate_mul().unwrap();
                Mul(
                    gate.type_id(),
                    gate.out_id(),
                    gate.left_id(),
                    gate.right_id(),
                )
            }

            ds::GateAddConstant => {
                let gate = gen_gate.directive_as_gate_add_constant().unwrap();
                AddConstant(
                    gate.type_id(),
                    gate.out_id(),
                    gate.in_id(),
                    Vec::from(gate.constant().ok_or("Missing constant")?),
                )
            }

            ds::GateMulConstant => {
                let gate = gen_gate.directive_as_gate_mul_constant().unwrap();
                MulConstant(
                    gate.type_id(),
                    gate.out_id(),
                    gate.in_id(),
                    Vec::from(gate.constant().ok_or("Missing constant")?),
                )
            }

            ds::GatePublicInput => {
                let gate = gen_gate.directive_as_gate_public_input().unwrap();
                PublicInput(gate.type_id(), gate.out_id())
            }

            ds::GatePrivateInput => {
                let gate = gen_gate.directive_as_gate_private_input().unwrap();
                PrivateInput(gate.type_id(), gate.out_id())
            }

            ds::GateNew => {
                let gate = gen_gate.directive_as_gate_new().unwrap();
                New(gate.type_id(), gate.first_id(), gate.last_id())
            }

            ds::GateDelete => {
                let gate = gen_gate.directive_as_gate_delete().unwrap();
                Delete(gate.type_id(), gate.first_id(), gate.last_id())
            }

            ds::GateConvert => {
                let gate = gen_gate.directive_as_gate_convert().unwrap();
                Convert(
                    WireList::try_from(gate.out_ids().ok_or("Missing outputs")?)?,
                    WireList::try_from(gate.in_ids().ok_or("Missing inputs")?)?,
                )
            }

            ds::GateCall => {
                let gate = gen_gate.directive_as_gate_call().unwrap();

                Call(
                    gate.name().ok_or("Missing function name.")?.into(),
                    WireList::try_from(gate.out_ids().ok_or("Missing outputs")?)?,
                    WireList::try_from(gate.in_ids().ok_or("Missing inputs")?)?,
                )
            }
        })
    }
}

impl Gate {
    /// Add this structure into a Flatbuffers message builder.
    pub fn build<'a>(
        &self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> WIPOffset<generated::Directive<'a>> {
        match self {
            Constant(type_id, output, constant) => {
                let g_constant = builder.create_vector(constant);

                let gate = generated::GateConstant::create(
                    builder,
                    &generated::GateConstantArgs {
                        type_id: *type_id,
                        out_id: *output,
                        constant: Some(g_constant),
                    },
                );
                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GateConstant,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            AssertZero(type_id, input) => {
                let gate = generated::GateAssertZero::create(
                    builder,
                    &generated::GateAssertZeroArgs {
                        type_id: *type_id,
                        in_id: *input,
                    },
                );
                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GateAssertZero,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            Copy(type_id, output, input) => {
                let gate = generated::GateCopy::create(
                    builder,
                    &generated::GateCopyArgs {
                        type_id: *type_id,
                        out_id: *output,
                        in_id: *input,
                    },
                );
                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GateCopy,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            Add(type_id, output, left, right) => {
                let gate = generated::GateAdd::create(
                    builder,
                    &generated::GateAddArgs {
                        type_id: *type_id,
                        out_id: *output,
                        left_id: *left,
                        right_id: *right,
                    },
                );
                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GateAdd,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            Mul(type_id, output, left, right) => {
                let gate = generated::GateMul::create(
                    builder,
                    &generated::GateMulArgs {
                        type_id: *type_id,
                        out_id: *output,
                        left_id: *left,
                        right_id: *right,
                    },
                );
                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GateMul,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            AddConstant(type_id, output, input, constant) => {
                let constant = builder.create_vector(constant);
                let gate = generated::GateAddConstant::create(
                    builder,
                    &generated::GateAddConstantArgs {
                        type_id: *type_id,
                        out_id: *output,
                        in_id: *input,
                        constant: Some(constant),
                    },
                );
                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GateAddConstant,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            MulConstant(type_id, output, input, constant) => {
                let constant = builder.create_vector(constant);
                let gate = generated::GateMulConstant::create(
                    builder,
                    &generated::GateMulConstantArgs {
                        type_id: *type_id,
                        out_id: *output,
                        in_id: *input,
                        constant: Some(constant),
                    },
                );
                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GateMulConstant,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            PublicInput(type_id, output) => {
                let gate = generated::GatePublicInput::create(
                    builder,
                    &generated::GatePublicInputArgs {
                        type_id: *type_id,
                        out_id: *output,
                    },
                );
                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GatePublicInput,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            PrivateInput(type_id, output) => {
                let gate = generated::GatePrivateInput::create(
                    builder,
                    &generated::GatePrivateInputArgs {
                        type_id: *type_id,
                        out_id: *output,
                    },
                );
                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GatePrivateInput,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            New(type_id, first, last) => {
                let gate = generated::GateNew::create(
                    builder,
                    &generated::GateNewArgs {
                        type_id: *type_id,
                        first_id: *first,
                        last_id: *last,
                    },
                );

                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GateNew,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            Delete(type_id, first, last) => {
                let gate = generated::GateDelete::create(
                    builder,
                    &generated::GateDeleteArgs {
                        type_id: *type_id,
                        first_id: *first,
                        last_id: *last,
                    },
                );

                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GateDelete,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            Convert(output, input) => {
                let g_output = build_wire_list(builder, output);
                let g_input = build_wire_list(builder, input);
                let gate = generated::GateConvert::create(
                    builder,
                    &generated::GateConvertArgs {
                        out_ids: Some(g_output),
                        in_ids: Some(g_input),
                    },
                );

                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GateConvert,
                        directive: Some(gate.as_union_value()),
                    },
                )
            }

            Call(name, output_wires, input_wires) => {
                let g_name = builder.create_string(name);
                let g_outputs = build_wire_list(builder, output_wires);
                let g_inputs = build_wire_list(builder, input_wires);

                let g_gate = generated::GateCall::create(
                    builder,
                    &generated::GateCallArgs {
                        name: Some(g_name),
                        out_ids: Some(g_outputs),
                        in_ids: Some(g_inputs),
                    },
                );

                generated::Directive::create(
                    builder,
                    &generated::DirectiveArgs {
                        directive_type: ds::GateCall,
                        directive: Some(g_gate.as_union_value()),
                    },
                )
            }
        }
    }

    /// Convert from a Flatbuffers vector of gates to owned structures.
    pub fn try_from_vector<'a>(
        g_vector: Vector<'a, ForwardsUOffset<generated::Directive<'a>>>,
    ) -> Result<Vec<Gate>> {
        let mut gates = vec![];
        for i in 0..g_vector.len() {
            let g_a = g_vector.get(i);
            gates.push(Gate::try_from(g_a)?);
        }
        Ok(gates)
    }

    /// Add a vector of this structure into a Flatbuffers message builder.
    pub fn build_vector<'a>(
        builder: &mut FlatBufferBuilder<'a>,
        gates: &[Gate],
    ) -> WIPOffset<Vector<'a, ForwardsUOffset<generated::Directive<'a>>>> {
        let g_gates: Vec<_> = gates.iter().map(|gate| gate.build(builder)).collect();
        builder.create_vector(&g_gates)
    }

    /// Returns the output wire id if exists.
    /// if not, returns None
    fn _get_output_wire_id(&self) -> Option<WireId> {
        match *self {
            Constant(_, w, _) => Some(w),
            Copy(_, w, _) => Some(w),
            Add(_, w, _, _) => Some(w),
            Mul(_, w, _, _) => Some(w),
            AddConstant(_, w, _, _) => Some(w),
            MulConstant(_, w, _, _) => Some(w),
            PublicInput(_, w) => Some(w),
            PrivateInput(_, w) => Some(w),

            AssertZero(_, _) => None,
            Delete(_, _, _) => None,
            New(_, _, _) => unimplemented!("New gate"),

            Convert(_, _) => unimplemented!("Convert gate"),
            Call(_, _, _) => unimplemented!("Call gate"),
        }
    }
}

/// replace_output_wires goes through all gates in `gates` and replace `output_wires[i]` by `i`.
/// If `output_wires[i]` belongs to a wire range (in New, Call, Convert gates),
/// add `Copy(i, output_wires[i])` at the end of gates and do not modify other gates containing `output_wires[i]`.
///
/// If a `Delete` gate contains an output wire, `replace_output_wires` will return an error.
pub fn replace_output_wires(gates: &mut Vec<Gate>, output_wires: &WireList) -> Result<()> {
    let expanded_output_wires = expand_wirelist(output_wires)?;
    let mut map: HashMap<TypeId, WireId> = HashMap::new();

    // It is not easily doable to replace a WireId in a wire range.
    // Therefor, if an output wire belongs to a wire range, we will add a Copy gate and not modify this WireId.
    let mut do_no_modify_wires: HashSet<(TypeId, WireId)> = HashSet::new();
    for gate in gates.iter() {
        match gate {
            New(type_id, first_id, last_id) => {
                for wire_id in *first_id..=*last_id {
                    do_no_modify_wires.insert((*type_id, wire_id));
                }
            }
            Call(_, out_ids, in_ids) => {
                expand_wirelist(out_ids)?
                    .iter()
                    .for_each(|(type_id, wire_id)| {
                        do_no_modify_wires.insert((*type_id, *wire_id));
                    });
                expand_wirelist(in_ids)?
                    .iter()
                    .for_each(|(type_id, wire_id)| {
                        do_no_modify_wires.insert((*type_id, *wire_id));
                    });
            }
            Convert(out_ids, in_ids) => {
                expand_wirelist(out_ids)?
                    .iter()
                    .for_each(|(type_id, wire_id)| {
                        do_no_modify_wires.insert((*type_id, *wire_id));
                    });
                expand_wirelist(in_ids)?
                    .iter()
                    .for_each(|(type_id, wire_id)| {
                        do_no_modify_wires.insert((*type_id, *wire_id));
                    });
            }
            _ => (),
        }
    }

    for (old_type_id, old_wire) in expanded_output_wires {
        let count = map.entry(old_type_id).or_insert(0);
        let new_wire = *count;
        *count += 1;

        // If the old_wire is in a wire range, we add a Copy gate and not modify this WireId in other gates.
        if do_no_modify_wires.contains(&(old_type_id, old_wire)) {
            gates.push(Copy(old_type_id, new_wire, old_wire));
            continue;
        }

        for gate in &mut *gates {
            match gate {
                Constant(ref type_id, ref mut output, _) => {
                    replace_wire_id(type_id, &old_type_id, output, old_wire, new_wire);
                }
                Copy(ref type_id, ref mut output, ref mut input) => {
                    replace_wire_id(type_id, &old_type_id, output, old_wire, new_wire);
                    replace_wire_id(type_id, &old_type_id, input, old_wire, new_wire);
                }
                Add(ref type_id, ref mut output, ref mut left, ref mut right) => {
                    replace_wire_id(type_id, &old_type_id, output, old_wire, new_wire);
                    replace_wire_id(type_id, &old_type_id, left, old_wire, new_wire);
                    replace_wire_id(type_id, &old_type_id, right, old_wire, new_wire);
                }
                Mul(ref type_id, ref mut output, ref mut left, ref mut right) => {
                    replace_wire_id(type_id, &old_type_id, output, old_wire, new_wire);
                    replace_wire_id(type_id, &old_type_id, left, old_wire, new_wire);
                    replace_wire_id(type_id, &old_type_id, right, old_wire, new_wire);
                }
                AddConstant(ref type_id, ref mut output, ref mut input, _) => {
                    replace_wire_id(type_id, &old_type_id, output, old_wire, new_wire);
                    replace_wire_id(type_id, &old_type_id, input, old_wire, new_wire);
                }
                MulConstant(ref type_id, ref mut output, ref mut input, _) => {
                    replace_wire_id(type_id, &old_type_id, output, old_wire, new_wire);
                    replace_wire_id(type_id, &old_type_id, input, old_wire, new_wire);
                }
                PublicInput(ref type_id, ref mut output) => {
                    replace_wire_id(type_id, &old_type_id, output, old_wire, new_wire);
                }
                PrivateInput(ref type_id, ref mut output) => {
                    replace_wire_id(type_id, &old_type_id, output, old_wire, new_wire);
                }
                AssertZero(ref type_id, ref mut wire) => {
                    replace_wire_id(type_id, &old_type_id, wire, old_wire, new_wire);
                }
                New(ref type_id, ref mut first, ref mut last) => {
                    // New gates have already been treated at the beginning of the loop
                    // by adding Copy gate if (old_type_id, old_wire) belongs to the New gate.
                    if (*first <= old_wire && *last >= old_wire) && (*type_id == old_type_id) {
                        panic!("Unreachable case !");
                    }
                }
                Delete(ref type_id, ref mut first, ref mut last) => {
                    if (*first <= old_wire && *last >= old_wire) && (*type_id == old_type_id) {
                        return Err("It is forbidden to delete an output wire !".into());
                    }
                }
                // Convert gates have already been treated at the beginning of the loop
                // by adding Copy gate if (old_type_id, old_wire) belongs to the Convert gate.
                Convert(_, _) => (),
                // Call gates have already been treated at the beginning of the loop
                // by adding Copy gate if (old_type_id, old_wire) belongs to the Call gate.
                Call(_, _, _) => (),
            }
        }
    }
    Ok(())
}

#[test]
fn test_replace_output_wires() {
    use crate::structs::wire::WireListElement::*;

    let mut gates = vec![
        New(0, 4, 4),
        PublicInput(0, 4),
        PrivateInput(0, 5),
        Constant(0, 6, vec![15]),
        PublicInput(1, 6),
        Add(0, 7, 4, 5),
        Delete(0, 4, 4),
        Mul(0, 8, 6, 7),
        Call(
            "custom".to_string(),
            vec![WireRange(0, 9, 12)],
            vec![WireRange(0, 7, 8)],
        ),
        AssertZero(0, 12),
    ];
    let output_wires = vec![WireRange(0, 4, 6), Wire(0, 12)];
    replace_output_wires(&mut gates, &output_wires).unwrap();
    let correct_gates = vec![
        New(0, 4, 4),
        PublicInput(0, 4),
        PrivateInput(0, 1),
        Constant(0, 2, vec![15]),
        PublicInput(1, 6),
        Add(0, 7, 4, 1),
        Delete(0, 4, 4),
        Mul(0, 8, 2, 7),
        Call(
            "custom".to_string(),
            vec![WireRange(0, 9, 12)],
            vec![WireRange(0, 7, 8)],
        ),
        AssertZero(0, 12),
        Copy(0, 0, 4),
        Copy(0, 3, 12),
    ];
    assert_eq!(gates, correct_gates);
}

#[test]
fn test_replace_output_wires_with_forbidden_delete() {
    use crate::structs::wire::WireListElement::*;

    let mut gates = vec![
        Add(0, 2, 4, 6),
        Mul(0, 7, 4, 6),
        Add(0, 8, 3, 5),
        Add(0, 9, 7, 8),
        Mul(0, 10, 3, 5),
        AddConstant(0, 11, 10, vec![1]),
        Delete(0, 7, 9),
    ];
    let output_wires = vec![Wire(0, 8), Wire(0, 4)];
    let test = replace_output_wires(&mut gates, &output_wires);
    assert!(test.is_err());

    let mut gates = vec![
        Add(0, 2, 4, 6),
        Mul(0, 7, 4, 6),
        Delete(0, 4, 4),
        Add(0, 8, 3, 5),
        Add(0, 9, 7, 8),
        Mul(0, 10, 3, 5),
        AddConstant(0, 11, 10, vec![1]),
    ];
    let output_wires = vec![Wire(0, 8), Wire(0, 4)];
    let test = replace_output_wires(&mut gates, &output_wires);
    assert!(test.is_err());
}
