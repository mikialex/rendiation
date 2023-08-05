use __core::num::NonZeroU32;
use fast_hash_collection::*;
use naga::Span;
use shadergraph::*;

pub struct ShaderAPINagaImpl {
  module: naga::Module,
  handle_id: usize,
  building_fn: naga::Function,
  block: Vec<naga::Block>,
  consts: FastHashMap<usize, naga::Handle<naga::Constant>>,
}

impl Default for ShaderAPINagaImpl {
  fn default() -> Self {
    let module = naga::Module::default();
    Self {
      module,
      handle_id: 0,
      block: Default::default(),
      building_fn: Default::default(),
      consts: Default::default(),
    }
  }
}

impl ShaderAPINagaImpl {
  fn get_expression(&self, handle: ShaderGraphNodeRawHandle) -> naga::Expression {
    todo!()
  }

  fn push_const(&mut self, constant: ConstNode) -> naga::Handle<naga::Constant> {
    match constant.data {
      PrimitiveShaderValue::Bool(v) => naga::Expression::Literal(naga::Literal::Bool(v)),
      PrimitiveShaderValue::Uint32(v) => naga::Expression::Literal(naga::Literal::U32(v)),
      PrimitiveShaderValue::Int32(v) => naga::Expression::Literal(naga::Literal::I32(v)),
      PrimitiveShaderValue::Float32(v) => naga::Expression::Literal(naga::Literal::F32(v)),
      PrimitiveShaderValue::Vec2Float32(v) => naga::Expression::Compose(naga::Literal::F32(v)),
      PrimitiveShaderValue::Vec3Float32(v) => todo!(),
      PrimitiveShaderValue::Vec4Float32(v) => todo!(),
      PrimitiveShaderValue::Vec2Uint32(v) => todo!(),
      PrimitiveShaderValue::Vec3Uint32(v) => todo!(),
      PrimitiveShaderValue::Vec4Uint32(v) => todo!(),
      PrimitiveShaderValue::Mat2Float32(v) => todo!(),
      PrimitiveShaderValue::Mat3Float32(v) => todo!(),
      PrimitiveShaderValue::Mat4Float32(v) => todo!(),
    }
    self.module.constants.append(value, Span::UNDEFINED)
  }
}

fn map_binary_op(o: BinaryOperator) -> naga::BinaryOperator {
  match o {
    BinaryOperator::Add => naga::BinaryOperator::Add,
    BinaryOperator::Sub => naga::BinaryOperator::Subtract,
    BinaryOperator::Mul => naga::BinaryOperator::Multiply,
    BinaryOperator::Div => naga::BinaryOperator::Divide,
    BinaryOperator::Rem => naga::BinaryOperator::Modulo,
    BinaryOperator::Eq => naga::BinaryOperator::Equal,
    BinaryOperator::NotEq => naga::BinaryOperator::NotEqual,
    BinaryOperator::GreaterThan => naga::BinaryOperator::Greater,
    BinaryOperator::LessThan => naga::BinaryOperator::Less,
    BinaryOperator::GreaterEqualThan => naga::BinaryOperator::GreaterEqual,
    BinaryOperator::LessEqualThan => naga::BinaryOperator::LessEqual,
    BinaryOperator::LogicalOr => naga::BinaryOperator::LogicalOr,
    BinaryOperator::LogicalAnd => naga::BinaryOperator::LogicalAnd,
  }
}

#[rustfmt::skip]
fn map_primitive_type(t: PrimitiveShaderValueType) -> naga::TypeInner {
  use PrimitiveShaderValueType::*;
  use naga::TypeInner::*;
  use naga::ScalarKind::*;
  use naga::VectorSize::*;

  match t {
    PrimitiveShaderValueType::Bool => Scalar { kind: naga::ScalarKind::Bool, width: 4 }, // bool is 4 bytes?
    Int32 => Scalar { kind: Sint, width: 4 },
    Uint32 => Scalar { kind: Uint, width: 4 },
    Float32 => Scalar { kind: Float, width: 4 },
    Vec2Float32 => Vector { size: Bi, kind:  Float, width: 4 },
    Vec3Float32 => Vector { size: Tri, kind:  Float, width: 4 },
    Vec4Float32 => Vector { size: Quad, kind:  Float, width: 4 },
    Vec2Uint32 => Vector { size: Bi, kind:  Uint, width: 4 },
    Vec3Uint32 => Vector { size: Tri, kind:  Uint, width: 4 },
    Vec4Uint32 => Vector { size: Quad, kind:  Uint, width: 4 },
    Mat2Float32 => Matrix { columns: Bi, rows: Bi, width: 4 },
    Mat3Float32 => Matrix { columns: Tri, rows: Tri, width: 4 },
    Mat4Float32 => Matrix { columns: Quad, rows: Quad, width: 4 },
}
}

impl ShaderAPI for ShaderAPINagaImpl {
  fn register_ty(&mut self, ty: ShaderValueType) {
    match ty {
      ShaderValueType::Single(v) => match v {
        ShaderValueSingleType::Fixed(f) => match f {
          ShaderStructMemberValueType::Primitive(p) => map_primitive_type(p),
          ShaderStructMemberValueType::Struct(st) => {
            let members = st
              .fields
              .iter()
              .map(|field| {
                //
                naga::StructMember {
                  name: String::from(field.name).into(),
                  ty: todo!(),
                  binding: None,
                  offset: todo!(),
                };
                todo!();
              })
              .collect();
            naga::TypeInner::Struct {
              members,
              span: Span::UNDEFINED,
            }
          }
          ShaderStructMemberValueType::FixedSizeArray((ty, size)) => naga::TypeInner::Array {
            base: todo!(),
            size: naga::ArraySize::Constant(NonZeroU32::new(size as u32).unwrap()),
            stride: todo!(),
          },
        },
        ShaderValueSingleType::Unsized(_) => todo!(),
        ShaderValueSingleType::Sampler(sampler) => naga::TypeInner::Sampler { comparison: false },
        ShaderValueSingleType::CompareSampler => naga::TypeInner::Sampler { comparison: true },
        ShaderValueSingleType::Texture {
          dimension,
          sample_type,
        } => {
          let (dim, arrayed) = match dimension {
            TextureViewDimension::D1 => (naga::ImageDimension::D1, false),
            TextureViewDimension::D2 => (naga::ImageDimension::D2, false),
            TextureViewDimension::D2Array => (naga::ImageDimension::D2, true),
            TextureViewDimension::Cube => (naga::ImageDimension::Cube, false),
            TextureViewDimension::CubeArray => (naga::ImageDimension::Cube, true),
            TextureViewDimension::D3 => (naga::ImageDimension::D3, false),
          };
          naga::TypeInner::Image {
            dim,
            arrayed,
            class: todo!(),
          }
        }
      },
      ShaderValueType::BindingArray { count, ty } => naga::TypeInner::BindingArray {
        base: todo!(),
        size: naga::ArraySize::Constant(NonZeroU32::new(count as u32).unwrap()),
      },
      ShaderValueType::Never => todo!(),
    };
    todo!()
  }

  fn make_expression(&mut self, expr: ShaderGraphNodeExpr) -> ShaderGraphNodeRawHandle {
    match expr {
      ShaderGraphNodeExpr::FunctionCall { meta, parameters } => todo!(),
      ShaderGraphNodeExpr::TextureSampling {
        texture,
        sampler,
        position,
        index,
        level,
      } => todo!(),
      ShaderGraphNodeExpr::Swizzle { ty, source } => todo!(),
      ShaderGraphNodeExpr::Compose { target, parameters } => todo!(),
      ShaderGraphNodeExpr::MatShrink { source, dimension } => todo!(),
      ShaderGraphNodeExpr::Operator(op) => match op {
        OperatorNode::Unary { one, operator } => todo!(),
        OperatorNode::Binary {
          left,
          right,
          operator,
        } => {
          let left = self.get_expression(left);
          let right = self.get_expression(right);
          let op = map_binary_op(operator);
        }
        OperatorNode::Index { array, entry } => todo!(),
      },
      ShaderGraphNodeExpr::FieldGet {
        field_name,
        struct_node,
      } => todo!(),
      ShaderGraphNodeExpr::StructConstruct { meta, fields } => naga::Expression::Compose {
        ty: (),
        components: (),
      },
      ShaderGraphNodeExpr::Const(c) => {
        // let handle = self.module.constants.append(value, Span::UNDEFINED);
      }
      ShaderGraphNodeExpr::Copy(_) => todo!(),
    }
  }

  fn define_input(&mut self, input: ShaderGraphInputNode) -> ShaderGraphNodeRawHandle {
    todo!()
  }

  fn push_scope(&mut self) {
    self.block.push(naga::Block::default())
  }

  fn pop_scope(&mut self) {
    let b = self.block.pop().unwrap();
    self
      .building_fn
      .body
      .push(naga::Statement::Block(b), Span::UNDEFINED);
  }

  fn push_if_scope(&mut self, condition: ShaderGraphNodeRawHandle) {
    todo!()
  }

  fn discard(&mut self) {
    self
      .building_fn
      .body
      .push(naga::Statement::Kill, Span::UNDEFINED)
  }

  fn push_for_scope(&mut self, target: ShaderIterator) -> ForNodes {
    // self.block.push(naga::Block::default())
    // self.block.last().unwrap().push(naga::Statement::If { condition: (), accept: (), reject: ()
    // }, span)
  }

  fn do_continue(&mut self, looper: ShaderGraphNodeRawHandle) {
    let st = naga::Statement::Continue;
    self.block.last_mut().unwrap().push(st, Span::UNDEFINED);
  }

  fn do_break(&mut self, looper: ShaderGraphNodeRawHandle) {
    let st = naga::Statement::Break;
    self.block.last_mut().unwrap().push(st, Span::UNDEFINED);
  }

  fn make_var(&mut self, ty: ShaderValueType) -> ShaderGraphNodeRawHandle {
    let v = naga::LocalVariable {
      name: None,
      ty: todo!(),
      init: None,
    };
    self.building_fn.local_variables.append(v, Span::UNDEFINED);
  }

  fn write(&mut self, source: ShaderGraphNodeRawHandle, target: ShaderGraphNodeRawHandle) {
    naga::Statement::Store {
      pointer: (),
      value: (),
    };
    todo!()
  }
  fn load(&mut self, source: ShaderGraphNodeRawHandle) -> ShaderGraphNodeRawHandle {
    // naga::Statement
    todo!()
  }

  fn build(&mut self) -> (String, String) {
    use naga::back::wgsl;

    // validate the IR
    let info = naga::valid::Validator::new(
      naga::valid::ValidationFlags::all(),
      naga::valid::Capabilities::all(),
    )
    .validate(&self.module)
    .unwrap();

    let wgsl = wgsl::write_string(&self.module, &info, wgsl::WriterFlags::empty()).unwrap();

    (wgsl, "main".to_owned())
  }

  fn define_frag_out(&mut self, idx: usize) -> ShaderGraphNodeRawHandle {
    todo!()
  }
}
