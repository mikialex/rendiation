use crate::*;

impl ShaderAPINagaImpl {
  pub(crate) fn make_expression_impl(&mut self, expr: ShaderNodeExpr) -> ShaderNodeRawHandle {
    let expr = match expr {
      ShaderNodeExpr::Fake => return ShaderNodeRawHandle { handle: 0 },
      ShaderNodeExpr::Zeroed { target } => {
        naga::Expression::ZeroValue(self.register_sized_ty(target))
      }
      ShaderNodeExpr::AtomicCall {
        ty,
        pointer,
        function,
        value,
      } => {
        let compare = match function {
          AtomicFunction::Exchange { compare, .. } => compare.map(|c| self.get_expression(c)),
          _ => None,
        };
        let comparison = compare.is_some();
        let fun = map_atomic_function(function, compare);

        let ty = if let AtomicFunction::Exchange { weak: true, .. } = function {
          self.module.generate_predeclared_type(
            naga::PredeclaredType::AtomicCompareExchangeWeakResult(map_atomic_scalar(ty)),
          )
        } else {
          let primitive = match ty {
            ShaderAtomicValueType::I32 => PrimitiveShaderValueType::i32(),
            ShaderAtomicValueType::U32 => PrimitiveShaderValueType::u32(),
          };
          self.register_primitive_ty(primitive)
        };

        return self.make_statement_result(
          naga::Expression::AtomicResult { ty, comparison },
          |this, result| naga::Statement::Atomic {
            pointer: this.get_expression(pointer),
            fun,
            value: this.get_expression(value),
            result: Some(result),
          },
        );
      }
      ShaderNodeExpr::FunctionCall {
        meta: ShaderFunctionType::Custom(meta),
        parameters,
      } => {
        let function = self.fn_mapping[&meta.name];
        return self.make_statement_result(
          naga::Expression::CallResult(function),
          |this, result| naga::Statement::Call {
            function,
            arguments: parameters.iter().map(|p| this.get_expression(*p)).collect(),
            result: Some(result),
          },
        );
      }
      ShaderNodeExpr::FunctionCall {
        meta: ShaderFunctionType::BuiltIn(f),
        parameters,
      } => self.lower_builtin_call(f, &parameters),
      ShaderNodeExpr::TextureQuery(texture, info) => {
        let level = match info {
          TextureQuery::Size { level } => level.map(|v| self.get_expression(v)),
          _ => None,
        };
        naga::Expression::ImageQuery {
          image: self.get_expression(texture),
          query: map_texture_query(info, level),
        }
      }
      ShaderNodeExpr::TextureSampling(ShaderTextureSampling {
        texture,
        sampler,
        position,
        array_index,
        level,
        reference,
        offset,
        gather_channel,
        clamp_to_edge,
      }) => naga::Expression::ImageSample {
        image: self.get_expression(texture),
        sampler: self.get_expression(sampler),
        gather: gather_channel.map(map_gather_channel),
        coordinate: self.get_expression(position),
        array_index: array_index.map(|index| self.get_expression(index)),
        offset: offset.map(|offset| {
          let data = PrimitiveShaderValue::from(offset);
          let constant = self.define_const_impl(
            ShaderStructFieldInitValue::Primitive(data),
            ShaderSizedValueType::Primitive(PrimitiveShaderValueType::vector(
              VectorSize::Bi,
              ScalarType::I32,
            )),
            true,
          );
          self.building_fn_mut().constant_expr(constant)
        }),
        level: map_sample_level(level, |handle| self.get_expression(handle)),
        depth_ref: reference.map(|r| self.get_expression(r)),
        clamp_to_edge,
      },
      ShaderNodeExpr::TextureLoad(ShaderTextureLoad {
        texture,
        position,
        array_index,
        level,
        sample_index,
      }) => naga::Expression::ImageLoad {
        image: self.get_expression(texture),
        coordinate: self.get_expression(position),
        array_index: array_index.map(|index| self.get_expression(index)),
        level: level.map(|level| self.get_expression(level)),
        sample: sample_index.map(|sample_index| self.get_expression(sample_index)),
      },
      ShaderNodeExpr::Swizzle {
        source,
        size,
        pattern,
      } => naga::Expression::Swizzle {
        size: map_vector_size(size),
        vector: self.get_expression(source),
        pattern: pattern.map(|component| match component {
          0 => naga::SwizzleComponent::X,
          1 => naga::SwizzleComponent::Y,
          2 => naga::SwizzleComponent::Z,
          3 => naga::SwizzleComponent::W,
          _ => unreachable!("invalid swizzle component"),
        }),
      },
      ShaderNodeExpr::Convert {
        source,
        convert_to,
        convert,
      } => naga::Expression::As {
        expr: self.get_expression(source),
        kind: map_scalar_kind(convert_to),
        convert,
      },
      ShaderNodeExpr::Compose { target, parameters } => {
        let components = parameters
          .iter()
          .map(|f| self.get_compose_component(*f))
          .collect();
        let ty = self.register_sized_ty(target);
        let components = self.fill_struct_padding_components(ty, components, Self::append_fn_expr);
        naga::Expression::Compose { ty, components }
      }
      ShaderNodeExpr::Derivative { axis, ctrl, source } => naga::Expression::Derivative {
        axis: map_derivative_axis(axis),
        ctrl: map_derivative_control(ctrl),
        expr: self.get_expression(source),
      },
      ShaderNodeExpr::Operator(op) => match op {
        OperatorNode::Unary { one, operator } => naga::Expression::Unary {
          op: map_unary_operator(operator),
          expr: self.get_expression(one),
        },
        OperatorNode::Binary {
          left,
          right,
          operator,
        } => naga::Expression::Binary {
          op: map_binary_op(operator),
          left: self.get_expression(left),
          right: self.get_expression(right),
        },
        OperatorNode::Index { array, entry } => naga::Expression::Access {
          base: self.get_expression(array),
          index: self.get_expression(entry),
        },
      },
      ShaderNodeExpr::IndexStatic {
        field_index,
        target: struct_node,
      } => {
        let base = self.get_expression(struct_node);
        let index = self.map_struct_field_index(base, field_index);
        naga::Expression::AccessIndex { base, index }
      }
      ShaderNodeExpr::RayQueryProceed { ray_query } => {
        return self.make_statement_result(
          naga::Expression::RayQueryProceedResult,
          |this, result| naga::Statement::RayQuery {
            query: this.get_expression(ray_query),
            fun: RayQueryFunction::Proceed { result },
          },
        );
      }
      ShaderNodeExpr::RayQueryGetCandidateIntersection { ray_query } => {
        self.module.generate_ray_intersection_type();
        naga::Expression::RayQueryGetIntersection {
          query: self.get_expression(ray_query),
          committed: false,
        }
      }
      ShaderNodeExpr::RayQueryGetCommittedIntersection { ray_query } => {
        self.module.generate_ray_intersection_type();
        naga::Expression::RayQueryGetIntersection {
          query: self.get_expression(ray_query),
          committed: true,
        }
      }
      ShaderNodeExpr::WorkGroupUniformLoad { pointer, ty } => {
        let ty = self.register_sized_ty(ty);
        return self.make_statement_result(
          naga::Expression::WorkGroupUniformLoadResult { ty },
          |this, result| naga::Statement::WorkGroupUniformLoad {
            pointer: this.get_expression(pointer),
            result,
          },
        );
      }
      ShaderNodeExpr::SubgroupBallot { predicate } => {
        return self.make_statement_result(
          naga::Expression::SubgroupBallotResult,
          |this, result| naga::Statement::SubgroupBallot {
            predicate: Some(this.get_expression(predicate)),
            result,
          },
        );
      }
      ShaderNodeExpr::SubgroupCollectiveOperation {
        operation,
        collective_operation,
        argument,
        ty,
      } => {
        let ty = self.register_primitive_ty(ty);
        return self.make_statement_result(
          naga::Expression::SubgroupOperationResult { ty },
          |this, result| naga::Statement::SubgroupCollectiveOperation {
            op: map_subgroup_operation(operation),
            collective_op: map_collective_operation(collective_operation),
            argument: this.get_expression(argument),
            result,
          },
        );
      }
      ShaderNodeExpr::SubgroupGather { mode, argument, ty } => {
        let ty = self.register_primitive_ty(ty);
        return self.make_statement_result(
          naga::Expression::SubgroupOperationResult { ty },
          |this, result| naga::Statement::SubgroupGather {
            mode: map_subgroup_gather_mode(mode, |handle| this.get_expression(handle)),
            argument: this.get_expression(argument),
            result,
          },
        );
      }
    };

    self.make_expression_inner(expr)
  }

  /// Create the node of the result expression produced by the statement, the result expression
  /// must not be emitted.
  fn make_statement_result(
    &mut self,
    result: naga::Expression,
    statement: impl FnOnce(&mut Self, naga::Handle<naga::Expression>) -> naga::Statement,
  ) -> ShaderNodeRawHandle {
    let result = self
      .building_fn_mut()
      .function
      .expressions
      .append(result, Span::UNDEFINED);
    let statement = statement(self, result);
    self.push_top_statement(statement);
    self.map_new_node(result)
  }

  fn lower_builtin_call(
    &mut self,
    f: ShaderBuiltInFunction,
    parameters: &[ShaderNodeRawHandle],
  ) -> naga::Expression {
    let args: Vec<_> = parameters.iter().map(|p| self.get_expression(*p)).collect();
    let relational = |fun| naga::Expression::Relational {
      fun,
      argument: args[0],
    };
    let math = |fun| naga::Expression::Math {
      fun,
      arg: args[0],
      arg1: args.get(1).copied(),
      arg2: args.get(2).copied(),
      arg3: args.get(3).copied(),
    };

    match f {
      ShaderBuiltInFunction::Select => naga::Expression::Select {
        condition: args[2],
        accept: args[1],
        reject: args[0],
      },
      ShaderBuiltInFunction::All => relational(naga::RelationalFunction::All),
      ShaderBuiltInFunction::Any => relational(naga::RelationalFunction::Any),
      ShaderBuiltInFunction::IsNan => relational(naga::RelationalFunction::IsNan),
      ShaderBuiltInFunction::IsInf => relational(naga::RelationalFunction::IsInf),
      ShaderBuiltInFunction::ArrayLength => naga::Expression::ArrayLength(args[0]),
      ShaderBuiltInFunction::Modf | ShaderBuiltInFunction::Frexp => {
        // the result struct type must be generated before use
        let arg_ty = self.resolve_expr_type(args[0]);
        let (size, scalar) = arg_ty
          .inner_with(&self.module.types)
          .vector_size_and_scalar()
          .expect("modf and frexp require float scalar or vector argument");
        let result_ty = if let ShaderBuiltInFunction::Modf = f {
          naga::PredeclaredType::ModfResult { size, scalar }
        } else {
          naga::PredeclaredType::FrexpResult { size, scalar }
        };
        self.module.generate_predeclared_type(result_ty);
        math(map_math_function(f))
      }
      f => math(map_math_function(f)),
    }
  }
}
