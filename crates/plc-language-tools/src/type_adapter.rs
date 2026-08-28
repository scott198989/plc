use plc_compiler::IrType;
use plc_program::DataType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeAdapterError {
    NamedType,
    Aggregate,
    BlockInstance,
    InstructionState,
}

/// Exhaustive adapter from the one canonical program type system into shared
/// compiler IR types. It intentionally rejects types that require layout or
/// state lowering rather than inventing a language-local type.
pub fn data_type_to_ir_type(data_type: &DataType) -> Result<IrType, TypeAdapterError> {
    match data_type {
        DataType::Bool => Ok(IrType::Bool),
        DataType::SInt => Ok(IrType::SInt),
        DataType::Int => Ok(IrType::Int),
        DataType::DInt => Ok(IrType::DInt),
        DataType::LInt => Ok(IrType::LInt),
        DataType::USInt => Ok(IrType::USInt),
        DataType::UInt => Ok(IrType::UInt),
        DataType::UDInt => Ok(IrType::UDInt),
        DataType::ULInt => Ok(IrType::ULInt),
        DataType::Byte => Ok(IrType::Byte),
        DataType::Word => Ok(IrType::Word),
        DataType::DWord => Ok(IrType::DWord),
        DataType::LWord => Ok(IrType::LWord),
        DataType::Real => Ok(IrType::Real),
        DataType::LReal => Ok(IrType::LReal),
        DataType::Char => Ok(IrType::Char),
        DataType::Time => Ok(IrType::Time),
        DataType::String { capacity } => Ok(IrType::String {
            capacity: *capacity,
        }),
        DataType::Named(_) => Err(TypeAdapterError::NamedType),
        DataType::Aggregate(_) => Err(TypeAdapterError::Aggregate),
        DataType::BlockInstance(_) => Err(TypeAdapterError::BlockInstance),
        DataType::InstructionState(_) => Err(TypeAdapterError::InstructionState),
    }
}
