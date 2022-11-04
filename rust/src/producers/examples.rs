use flatbuffers::{emplace_scalar, read_scalar, EndianScalar};
use std::collections::HashMap;
use std::mem::size_of;

use crate::structs::inputs::Inputs;
use crate::structs::wire::WireListElement;
use crate::{Header, PrivateInputs, PublicInputs, Relation, TypeId};

pub fn example_header() -> Header {
    example_header_in_type(literal32(EXAMPLE_MODULUS))
}

pub fn example_public_inputs() -> PublicInputs {
    example_public_inputs_h(&example_header())
}

pub fn example_private_inputs() -> PrivateInputs {
    example_private_inputs_h(&example_header())
}

pub fn example_private_inputs_incorrect() -> PrivateInputs {
    example_private_inputs_incorrect_h(&example_header())
}

pub fn example_relation() -> Relation {
    example_relation_h(&example_header())
}

pub fn example_header_in_type(modulo: Vec<u8>) -> Header {
    Header::new(&[modulo])
}

// pythogarean example
pub fn example_public_inputs_h(header: &Header) -> PublicInputs {
    PublicInputs {
        header: header.clone(),
        inputs: vec![Inputs {
            values: vec![literal32(5)],
        }],
    }
}

pub fn example_private_inputs_h(header: &Header) -> PrivateInputs {
    PrivateInputs {
        header: header.clone(),
        inputs: vec![Inputs {
            values: vec![literal32(3), literal32(4)],
        }],
    }
}

pub fn example_private_inputs_incorrect_h(header: &Header) -> PrivateInputs {
    PrivateInputs {
        header: header.clone(),
        inputs: vec![Inputs {
            values: vec![
                literal32(3),
                literal32(4 + 1), // incorrect.
            ],
        }],
    }
}

pub fn example_relation_h(header: &Header) -> Relation {
    use crate::structs::function::{Function, FunctionBody};
    use crate::Gate::*;

    let type_id: TypeId = 0;

    Relation {
        header: header.clone(),
        plugins: vec![],
        functions: vec![Function::new(
            "square".to_string(),
            HashMap::from([(type_id, 1)]),
            HashMap::from([(type_id, 1)]),
            HashMap::new(),
            HashMap::new(),
            FunctionBody::Gates(vec![Mul(type_id, 0, 1, 1)]),
        )],
        gates: vec![
            // Right-triangle example
            New(type_id, 0, 2),
            PublicInput(type_id, 0),
            PrivateInput(type_id, 1),
            PrivateInput(type_id, 2),
            Call(
                "square".to_string(),
                vec![WireListElement::Wire(type_id, 3)],
                vec![WireListElement::Wire(type_id, 0)],
            ),
            Call(
                "square".to_string(),
                vec![WireListElement::Wire(type_id, 4)],
                vec![WireListElement::Wire(type_id, 1)],
            ),
            Call(
                "square".to_string(),
                vec![WireListElement::Wire(type_id, 5)],
                vec![WireListElement::Wire(type_id, 2)],
            ),
            Add(type_id, 6, 4, 5),
            MulConstant(type_id, 7, 3, vec![100]),
            Add(type_id, 8, 6, 7),
            AssertZero(type_id, 8),
            Delete(type_id, 0, Some(2)),
            Delete(type_id, 3, Some(8)),
        ],
    }
}

pub const EXAMPLE_MODULUS: u32 = 101;

pub fn literal<T: EndianScalar>(value: T) -> Vec<u8> {
    let mut buf = vec![0u8; size_of::<T>()];
    emplace_scalar(&mut buf[..], value);
    buf
}

pub fn literal32(v: u32) -> Vec<u8> {
    literal(v)
}

pub fn read_literal<T: EndianScalar>(encoded: &[u8]) -> T {
    if encoded.len() >= size_of::<T>() {
        read_scalar(encoded)
    } else {
        let mut encoded = Vec::from(encoded);
        encoded.resize(size_of::<T>(), 0);
        read_scalar(&encoded)
    }
}

pub fn encode_negative_one(modulo: &[u8]) -> Vec<u8> {
    let mut neg_one = modulo.to_owned();
    assert!(!neg_one.is_empty() && neg_one[0] > 0, "Invalid modulo");
    neg_one[0] -= 1;
    neg_one
}

#[test]
fn test_examples() {
    use crate::Source;

    let mut common_buf = Vec::<u8>::new();
    example_public_inputs().write_into(&mut common_buf).unwrap();
    example_relation().write_into(&mut common_buf).unwrap();

    let mut prover_buf = Vec::<u8>::new();
    example_private_inputs()
        .write_into(&mut prover_buf)
        .unwrap();

    let source = Source::from_buffers(vec![common_buf, prover_buf]);
    let messages = source.read_all_messages().unwrap();
    assert_eq!(messages.relations, vec![example_relation()]);
    assert_eq!(messages.public_inputs, vec![example_public_inputs()]);
    assert_eq!(messages.private_inputs, vec![example_private_inputs()]);
}
