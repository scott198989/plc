use alloc::{collections::BTreeMap, vec::Vec};

use plc_program::{BlockId, DataType};

use crate::{
    BinaryOperator, IrBasicBlock, IrBasicBlockId, IrFunction, IrOperation, IrOperationId,
    IrOperationKind, IrTerminator, IrTerminatorKind, IrType, IrValue, IrValueId, ProbeDefinition,
    ProbeId, ProbeKind, ProbeTable, SclSource, SourceAnchor, SourceLanguage, SourceMapEntry,
    SourceMapId, SourceMapSite, SourceMapTable, TYPED_IR_VERSION, TypedIrProgram, UnaryOperator,
    scl::{
        BinaryOp, TypedBlock, TypedExpr, TypedExprKind, TypedStatement, TypedStatementKind, UnaryOp,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LoweringError {
    UnsupportedType(DataType),
    ErrorNode,
    IdentityOverflow,
    MissingBlock(IrBasicBlockId),
    DuplicateTerminator(IrBasicBlockId),
}

pub(crate) struct LoweredProgram {
    pub ir: TypedIrProgram,
    pub source_maps: SourceMapTable,
    pub probes: ProbeTable,
    pub operation_count: usize,
}

pub(crate) fn lower_typed_blocks(
    blocks: &[(TypedBlock, SclSource)],
) -> Result<LoweredProgram, LoweringError> {
    let mut context = LoweringContext {
        next_operation: 1,
        next_value: 1,
        next_source_map: 1,
        next_probe: 1,
        source_maps: BTreeMap::new(),
        probes: BTreeMap::new(),
        operation_count: 0,
    };
    let mut functions = BTreeMap::new();
    for (typed, source) in blocks {
        let function = context.lower_function(typed, source)?;
        functions.insert(typed.owner, function);
    }
    Ok(LoweredProgram {
        ir: TypedIrProgram::from_untrusted_parts(TYPED_IR_VERSION, functions),
        source_maps: SourceMapTable::from_untrusted_entries(context.source_maps),
        probes: ProbeTable::from_untrusted_entries(context.probes),
        operation_count: context.operation_count,
    })
}

struct LoweringContext {
    next_operation: u32,
    next_value: u32,
    next_source_map: u32,
    next_probe: u32,
    source_maps: BTreeMap<SourceMapId, SourceMapEntry>,
    probes: BTreeMap<ProbeId, ProbeDefinition>,
    operation_count: usize,
}

impl LoweringContext {
    fn lower_function(
        &mut self,
        typed: &TypedBlock,
        source: &SclSource,
    ) -> Result<IrFunction, LoweringError> {
        let mut builder = FunctionBuilder::new(typed.owner, typed.kind, source, self)?;
        let entry = builder.new_block()?;
        let end = builder.lower_statements(&typed.statements, Some(entry))?;
        if let Some(end) = end {
            let causal = SourceAnchor {
                owner_object_id: typed.owner,
                source_revision_hash: source.revision_hash(),
                language: SourceLanguage::Scl,
                semantic_node_id: crate::SemanticNodeId::new(0),
                text_range: crate::TextRange {
                    start: 0,
                    end: u32::try_from(source.text().len()).unwrap_or(u32::MAX),
                },
            };
            builder.terminate(
                end,
                IrTerminatorKind::Return,
                causal,
                ProbeKind::Return,
                true,
            )?;
        }
        builder.finish(entry)
    }

    fn next_operation(&mut self) -> Result<IrOperationId, LoweringError> {
        let id = IrOperationId::new(self.next_operation);
        self.next_operation = self
            .next_operation
            .checked_add(1)
            .ok_or(LoweringError::IdentityOverflow)?;
        self.operation_count = self.operation_count.saturating_add(1);
        Ok(id)
    }

    fn next_value(&mut self) -> Result<IrValueId, LoweringError> {
        let id = IrValueId::new(self.next_value);
        self.next_value = self
            .next_value
            .checked_add(1)
            .ok_or(LoweringError::IdentityOverflow)?;
        Ok(id)
    }

    fn map_and_probe(
        &mut self,
        site: SourceMapSite,
        anchor: SourceAnchor,
        kind: ProbeKind,
        value_type: Option<IrType>,
        compiler_generated: bool,
    ) -> Result<(SourceMapId, ProbeId), LoweringError> {
        let source_map = SourceMapId::new(self.next_source_map);
        self.next_source_map = self
            .next_source_map
            .checked_add(1)
            .ok_or(LoweringError::IdentityOverflow)?;
        let probe = ProbeId::new(self.next_probe);
        self.next_probe = self
            .next_probe
            .checked_add(1)
            .ok_or(LoweringError::IdentityOverflow)?;
        self.source_maps.insert(
            source_map,
            SourceMapEntry {
                id: source_map,
                site,
                anchors: alloc::vec![anchor],
                compiler_generated,
            },
        );
        self.probes.insert(
            probe,
            ProbeDefinition {
                id: probe,
                site,
                kind,
                value_type,
                source_map,
            },
        );
        Ok((source_map, probe))
    }
}

struct PartialBlock {
    operations: Vec<IrOperation>,
    terminator: Option<IrTerminator>,
}

struct FunctionBuilder<'a, 'b> {
    owner: BlockId,
    kind: plc_program::ProgramUnitKind,
    source: &'a SclSource,
    context: &'b mut LoweringContext,
    next_block: u32,
    blocks: BTreeMap<IrBasicBlockId, PartialBlock>,
}

impl<'a, 'b> FunctionBuilder<'a, 'b> {
    fn new(
        owner: BlockId,
        kind: plc_program::ProgramUnitKind,
        source: &'a SclSource,
        context: &'b mut LoweringContext,
    ) -> Result<Self, LoweringError> {
        if !kind.is_executable() {
            return Err(LoweringError::ErrorNode);
        }
        Ok(Self {
            owner,
            kind,
            source,
            context,
            next_block: 1,
            blocks: BTreeMap::new(),
        })
    }

    fn new_block(&mut self) -> Result<IrBasicBlockId, LoweringError> {
        let id = IrBasicBlockId::new(self.next_block);
        self.next_block = self
            .next_block
            .checked_add(1)
            .ok_or(LoweringError::IdentityOverflow)?;
        self.blocks.insert(
            id,
            PartialBlock {
                operations: Vec::new(),
                terminator: None,
            },
        );
        Ok(id)
    }

    fn lower_statements(
        &mut self,
        statements: &[TypedStatement],
        mut current: Option<IrBasicBlockId>,
    ) -> Result<Option<IrBasicBlockId>, LoweringError> {
        for statement in statements {
            let Some(block) = current else {
                break;
            };
            current = match &statement.kind {
                TypedStatementKind::Assignment { target, value } => {
                    let value_id = self.lower_expr(block, value)?;
                    self.emit(
                        block,
                        None,
                        IrOperationKind::StoreMember {
                            target: target.id,
                            value: value_id,
                        },
                        self.anchor(statement.id, statement.range),
                        ProbeKind::StorageWrite,
                    )?;
                    Some(block)
                }
                TypedStatementKind::If {
                    branches,
                    else_body,
                } => self.lower_if(block, statement, branches, else_body)?,
                TypedStatementKind::Return => {
                    self.terminate(
                        block,
                        IrTerminatorKind::Return,
                        self.anchor(statement.id, statement.range),
                        ProbeKind::Return,
                        false,
                    )?;
                    None
                }
                TypedStatementKind::Error => return Err(LoweringError::ErrorNode),
            };
        }
        Ok(current)
    }

    fn lower_if(
        &mut self,
        first_condition_block: IrBasicBlockId,
        statement: &TypedStatement,
        branches: &[(TypedExpr, Vec<TypedStatement>)],
        else_body: &[TypedStatement],
    ) -> Result<Option<IrBasicBlockId>, LoweringError> {
        let merge = self.new_block()?;
        let mut condition_block = first_condition_block;
        for (index, (condition, body)) in branches.iter().enumerate() {
            let condition_value = self.lower_expr(condition_block, condition)?;
            let when_true = self.new_block()?;
            let last = index + 1 == branches.len();
            let when_false = if last {
                if else_body.is_empty() {
                    merge
                } else {
                    self.new_block()?
                }
            } else {
                self.new_block()?
            };
            self.terminate(
                condition_block,
                IrTerminatorKind::Branch {
                    condition: condition_value,
                    when_true,
                    when_false,
                },
                self.anchor(condition.id, condition.range),
                ProbeKind::Branch,
                false,
            )?;
            if let Some(end) = self.lower_statements(body, Some(when_true))? {
                self.terminate(
                    end,
                    IrTerminatorKind::Jump(merge),
                    self.anchor(statement.id, statement.range),
                    ProbeKind::Branch,
                    true,
                )?;
            }
            condition_block = when_false;
        }
        if !else_body.is_empty()
            && let Some(end) = self.lower_statements(else_body, Some(condition_block))?
        {
            self.terminate(
                end,
                IrTerminatorKind::Jump(merge),
                self.anchor(statement.id, statement.range),
                ProbeKind::Branch,
                true,
            )?;
        }
        Ok(Some(merge))
    }

    fn lower_expr(
        &mut self,
        block: IrBasicBlockId,
        expression: &TypedExpr,
    ) -> Result<IrValueId, LoweringError> {
        let data_type = IrType::from_program_type(&expression.data_type)
            .ok_or_else(|| LoweringError::UnsupportedType(expression.data_type.clone()))?;
        match &expression.kind {
            TypedExprKind::Constant(value) => self
                .emit(
                    block,
                    Some(data_type),
                    IrOperationKind::Constant(value.clone()),
                    self.anchor(expression.id, expression.range),
                    ProbeKind::Constant,
                )?
                .ok_or(LoweringError::ErrorNode),
            TypedExprKind::Member(member) => self
                .emit(
                    block,
                    Some(data_type),
                    IrOperationKind::LoadMember { member: member.id },
                    self.anchor(expression.id, expression.range),
                    ProbeKind::StorageRead,
                )?
                .ok_or(LoweringError::ErrorNode),
            TypedExprKind::Unary { operator, operand } => {
                let operand = self.lower_expr(block, operand)?;
                self.emit(
                    block,
                    Some(data_type),
                    IrOperationKind::Unary {
                        operator: lower_unary(*operator),
                        operand,
                    },
                    self.anchor(expression.id, expression.range),
                    ProbeKind::Expression,
                )?
                .ok_or(LoweringError::ErrorNode)
            }
            TypedExprKind::Binary {
                operator,
                left,
                right,
            } => {
                // Left-to-right lowering is the explicit eager evaluation order.
                let left = self.lower_expr(block, left)?;
                let right = self.lower_expr(block, right)?;
                self.emit(
                    block,
                    Some(data_type),
                    IrOperationKind::Binary {
                        operator: lower_binary(*operator),
                        left,
                        right,
                    },
                    self.anchor(expression.id, expression.range),
                    ProbeKind::Expression,
                )?
                .ok_or(LoweringError::ErrorNode)
            }
            TypedExprKind::Error => Err(LoweringError::ErrorNode),
        }
    }

    fn emit(
        &mut self,
        block: IrBasicBlockId,
        result_type: Option<IrType>,
        kind: IrOperationKind,
        anchor: SourceAnchor,
        probe_kind: ProbeKind,
    ) -> Result<Option<IrValueId>, LoweringError> {
        let operation_id = self.context.next_operation()?;
        let result = match result_type {
            Some(data_type) => Some(IrValue {
                id: self.context.next_value()?,
                data_type,
            }),
            None => None,
        };
        let site = SourceMapSite {
            function: self.owner,
            basic_block: block,
            operation: Some(operation_id),
        };
        let (source_map, probe) = self.context.map_and_probe(
            site,
            anchor,
            probe_kind,
            result.as_ref().map(|value| value.data_type.clone()),
            false,
        )?;
        let value_id = result.as_ref().map(|value| value.id);
        let partial = self
            .blocks
            .get_mut(&block)
            .ok_or(LoweringError::MissingBlock(block))?;
        partial.operations.push(IrOperation {
            id: operation_id,
            result,
            kind,
            source_map,
            probe,
        });
        Ok(value_id)
    }

    fn terminate(
        &mut self,
        block: IrBasicBlockId,
        kind: IrTerminatorKind,
        anchor: SourceAnchor,
        probe_kind: ProbeKind,
        compiler_generated: bool,
    ) -> Result<(), LoweringError> {
        let value_type = if matches!(kind, IrTerminatorKind::Branch { .. }) {
            Some(IrType::Bool)
        } else {
            None
        };
        let site = SourceMapSite {
            function: self.owner,
            basic_block: block,
            operation: None,
        };
        let (source_map, probe) =
            self.context
                .map_and_probe(site, anchor, probe_kind, value_type, compiler_generated)?;
        let partial = self
            .blocks
            .get_mut(&block)
            .ok_or(LoweringError::MissingBlock(block))?;
        if partial.terminator.is_some() {
            return Err(LoweringError::DuplicateTerminator(block));
        }
        partial.terminator = Some(IrTerminator {
            kind,
            source_map,
            probe,
        });
        Ok(())
    }

    fn anchor(&self, node: crate::SemanticNodeId, range: crate::TextRange) -> SourceAnchor {
        SourceAnchor {
            owner_object_id: self.owner,
            source_revision_hash: self.source.revision_hash(),
            language: SourceLanguage::Scl,
            semantic_node_id: node,
            text_range: range,
        }
    }

    fn finish(self, entry: IrBasicBlockId) -> Result<IrFunction, LoweringError> {
        let mut blocks = BTreeMap::new();
        for (id, block) in self.blocks {
            blocks.insert(
                id,
                IrBasicBlock {
                    id,
                    operations: block.operations,
                    terminator: block.terminator.ok_or(LoweringError::MissingBlock(id))?,
                },
            );
        }
        Ok(IrFunction {
            owner: self.owner,
            source_kind: self.kind,
            entry,
            blocks,
        })
    }
}

const fn lower_unary(value: UnaryOp) -> UnaryOperator {
    match value {
        UnaryOp::Plus => UnaryOperator::Plus,
        UnaryOp::Minus => UnaryOperator::Negate,
        UnaryOp::Not => UnaryOperator::Not,
    }
}

const fn lower_binary(value: BinaryOp) -> BinaryOperator {
    match value {
        BinaryOp::Multiply => BinaryOperator::Multiply,
        BinaryOp::Divide => BinaryOperator::Divide,
        BinaryOp::Modulo => BinaryOperator::Modulo,
        BinaryOp::Add => BinaryOperator::Add,
        BinaryOp::Subtract => BinaryOperator::Subtract,
        BinaryOp::Equal => BinaryOperator::Equal,
        BinaryOp::NotEqual => BinaryOperator::NotEqual,
        BinaryOp::Less => BinaryOperator::Less,
        BinaryOp::LessEqual => BinaryOperator::LessEqual,
        BinaryOp::Greater => BinaryOperator::Greater,
        BinaryOp::GreaterEqual => BinaryOperator::GreaterEqual,
        BinaryOp::And => BinaryOperator::And,
        BinaryOp::Xor => BinaryOperator::Xor,
        BinaryOp::Or => BinaryOperator::Or,
    }
}
