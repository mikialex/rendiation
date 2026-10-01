use crate::*;

impl ShaderAPINagaImpl {
  fn append_global_expr(&mut self, expr: naga::Expression) -> naga::Handle<naga::Expression> {
    self.module.global_expressions.append(expr, Span::UNDEFINED)
  }

  /// Create the expressions of the primitive value in the global expression arena.
  fn global_primitive_expr(
    &mut self,
    data: PrimitiveShaderValue,
  ) -> naga::Handle<naga::Expression> {
    match data {
      PrimitiveShaderValue::Scalar(v) => {
        self.append_global_expr(naga::Expression::Literal(scalar_value_to_naga_literal(v)))
      }
      PrimitiveShaderValue::Vector { size, scalar, data } => {
        self.global_compose_scalars(PrimitiveShaderValueType::vector(size, scalar), data.iter())
      }
      PrimitiveShaderValue::Matrix {
        columns,
        rows,
        scalar,
        data,
      } => {
        // naga requires matrix compose from column vectors
        let column_ty = PrimitiveShaderValueType::vector(rows, scalar);
        let components = data
          .iter()
          .map(|column| self.global_compose_scalars(column_ty, column.iter()))
          .collect();
        self.global_compose(
          PrimitiveShaderValueType::Matrix {
            columns,
            rows,
            scalar,
          },
          components,
        )
      }
    }
  }

  fn global_compose_scalars<'a>(
    &mut self,
    ty: PrimitiveShaderValueType,
    scalars: impl Iterator<Item = &'a ScalarValue>,
  ) -> naga::Handle<naga::Expression> {
    let components = scalars
      .map(|v| self.append_global_expr(naga::Expression::Literal(scalar_value_to_naga_literal(*v))))
      .collect();
    self.global_compose(ty, components)
  }

  fn global_compose(
    &mut self,
    ty: PrimitiveShaderValueType,
    components: Vec<naga::Handle<naga::Expression>>,
  ) -> naga::Handle<naga::Expression> {
    let ty = self.register_primitive_ty(ty);
    self.append_global_expr(naga::Expression::Compose { ty, components })
  }

  fn define_const_global_expr_impl(
    &mut self,
    value: ShaderStructFieldInitValue,
    raw_ty: &ShaderSizedValueType,
  ) -> naga::Handle<naga::Expression> {
    match (value, raw_ty) {
      (ShaderStructFieldInitValue::Primitive(init), ShaderSizedValueType::Primitive(_)) => {
        self.global_primitive_expr(init)
      }
      (ShaderStructFieldInitValue::Struct(init), ShaderSizedValueType::Struct(meta)) => {
        let init: Vec<_> = init
          .iter()
          .zip(meta.fields.iter())
          .map(|(v, f_ty)| self.define_const_global_expr_impl(v.clone(), &f_ty.ty))
          .collect();
        let ty = self.register_sized_ty(raw_ty.clone());
        let components = self.fill_struct_padding_components(ty, init, Self::append_global_expr);
        self.append_global_expr(naga::Expression::Compose { ty, components })
      }
      (ShaderStructFieldInitValue::Array(init), ShaderSizedValueType::FixedSizeArray(f_ty, _)) => {
        let ty = self.register_sized_ty(raw_ty.clone());
        let components = init
          .iter()
          .map(|v| self.define_const_global_expr_impl(v.clone(), f_ty))
          .collect();
        self.append_global_expr(naga::Expression::Compose { ty, components })
      }
      _ => unreachable!("ty not match"),
    }
  }

  /// Define the constant, and create its expression in the building function immediately, so the
  /// expression indices of this function do not depend on where the constant is first used.
  pub(crate) fn define_const_impl(
    &mut self,
    value: ShaderStructFieldInitValue,
    ty: ShaderSizedValueType,
    inlined: bool,
  ) -> naga::Handle<naga::Constant> {
    let global_expr = self.define_const_global_expr_impl(value, &ty);

    let ty = self.register_sized_ty(ty);

    let constant = self.module.constants.append(
      naga::Constant {
        // this name should be set, or naga will inlined the const into function.
        name: if inlined {
          None
        } else {
          Some(format!("const{}", self.module.constants.len()))
        },
        ty,
        init: global_expr,
      },
      Span::UNDEFINED,
    );

    self.building_fn_mut().constant_expr(constant);
    constant
  }

  // root cause: this works around a bug in naga's spirv backend. when a compose
  // expression is const-folded into an OpConstantComposite, the backend flattens
  // nested compose/splat expressions but does not flatten Expression::Constant
  // components, so composing a constant value with scalars (e.g. vec4(v3_const, 1.0))
  // produces an OpConstantComposite whose constituent count does not match the vector
  // size, which spirv-val rejects. naga's own wgsl parser never hits this because it
  // deep-copies a constant's init expression into the function arena whenever the
  // constant is referenced in function code, so we mirror that behavior here.
  pub(crate) fn get_compose_component(
    &mut self,
    handle: ShaderNodeRawHandle,
  ) -> naga::Handle<naga::Expression> {
    match self.nodes[handle.handle] {
      NodeSlot::Constant(c) => self.inline_constant_value_into_fn(c),
      _ => self.get_expression(handle),
    }
  }

  fn inline_constant_value_into_fn(
    &mut self,
    constant: naga::Handle<naga::Constant>,
  ) -> naga::Handle<naga::Expression> {
    let init = self.module.constants[constant].init;
    self.copy_global_expr_into_fn(init)
  }

  fn copy_global_expr_into_fn(
    &mut self,
    handle: naga::Handle<naga::Expression>,
  ) -> naga::Handle<naga::Expression> {
    let expr = self.module.global_expressions[handle].clone();
    match expr {
      naga::Expression::Literal(_) | naga::Expression::ZeroValue(_) => self.append_fn_expr(expr),
      naga::Expression::Compose { ty, components } => {
        let components = components
          .iter()
          .map(|c| self.copy_global_expr_into_fn(*c))
          .collect();
        self.append_fn_expr(naga::Expression::Compose { ty, components })
      }
      naga::Expression::Constant(c) => self.copy_global_expr_into_fn(self.module.constants[c].init),
      other => unreachable!("unexpected global expression in constant init: {other:?}"),
    }
  }
}
