use __core::num::NonZeroU32;
use fast_hash_collection::*;
use naga::Span;
use shadergraph::*;

pub struct ShaderAPINagaImpl {
  module: naga::Module,
  handle_id: usize,
  building_fn: Vec<(String, naga::Function, usize)>,
  block: Vec<naga::Block>,
  fn_mapping: FastHashMap<String, (naga::Handle<naga::Function>, ShaderFunctionMetaInfo)>,
  consts_mapping: FastHashMap<ShaderGraphNodeRawHandle, naga::Handle<naga::Constant>>,
  expression_mapping: FastHashMap<ShaderGraphNodeRawHandle, naga::Handle<naga::Expression>>,
}

const ENTRY_POINT_NAME: &str = "main";

impl ShaderAPINagaImpl {
  pub fn new(stage: ShaderStages) -> Self {
    let stage = match stage {
      ShaderStages::Vertex => naga::ShaderStage::Vertex,
      ShaderStages::Fragment => naga::ShaderStage::Fragment,
    };

    let module = naga::Module::default();
    let entry = naga::EntryPoint {
      name: ENTRY_POINT_NAME.to_owned(),
      stage,
      early_depth_test: None,    // todo expose
      workgroup_size: [0, 0, 0], // todo expose , why naga not make this an enum??
      function: naga::Function {
        name: None,
        arguments: todo!(),
        result: todo!(),
        local_variables: todo!(),
        expressions: todo!(),
        named_expressions: todo!(),
        body: todo!(),
      },
    };
    module.entry_points.push(entry);

    Self {
      module,
      handle_id: 0,
      block: Default::default(),
      building_fn: Default::default(),
      fn_mapping: Default::default(),
      consts_mapping: Default::default(),
      expression_mapping: Default::default(),
    }
  }

  fn make_new_handle(&mut self) -> ShaderGraphNodeRawHandle {
    self.handle_id += 1;
    let handle = self.handle_id;
    ShaderGraphNodeRawHandle { handle }
  }

  fn make_expression_inner(&mut self, expr: naga::Expression) -> ShaderGraphNodeRawHandle {
    let handle = self
      .building_fn
      .last_mut()
      .unwrap()
      .1
      .expressions
      .append(expr, Span::UNDEFINED);

    // should we merge these expression emits?
    self.block.last_mut().unwrap().push(
      naga::Statement::Emit(naga::Range::new_from_bounds(handle, handle)),
      Span::UNDEFINED,
    );

    let return_handle = self.make_new_handle();
    self.expression_mapping.insert(return_handle, handle);
    return_handle
  }

  fn register_ty_impl(&mut self, ty: ShaderValueType) -> naga::Handle<naga::Type> {
    let ty = match ty {
      ShaderValueType::Single(v) => match v {
        ShaderValueSingleType::Sized(f) => match f {
          ShaderSizedValueType::Primitive(p) => map_primitive_type(p),
          ShaderSizedValueType::Struct(st) => {
            let members = st
              .fields
              .iter()
              .map(|field| {
                //
                naga::StructMember {
                  name: String::from(field.name).into(),
                  ty: self.register_ty_impl(ty),
                  binding: None,
                  offset: todo!(),
                }
              })
              .collect();
            naga::TypeInner::Struct {
              members,
              span: todo!(),
            }
          }
          ShaderSizedValueType::FixedSizeArray((ty, size)) => naga::TypeInner::Array {
            base: self.register_ty_impl(ShaderValueType::Single(ShaderValueSingleType::Sized(*ty))),
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
        base: self.register_ty_impl(ShaderValueType::Single(ty)),
        size: naga::ArraySize::Constant(NonZeroU32::new(count as u32).unwrap()),
      },
      ShaderValueType::Never => todo!(),
    };
    let ty = naga::Type {
      name: None,
      inner: ty,
    };
    self.module.types.insert(ty, Span::UNDEFINED)
  }

  fn get_expression(&self, handle: ShaderGraphNodeRawHandle) -> naga::Handle<naga::Expression> {
    *self.expression_mapping.get(&handle).unwrap()
  }

  fn push_const(&mut self, constant: ConstNode) -> naga::Handle<naga::Constant> {
    // match constant.data {
    //   PrimitiveShaderValue::Bool(v) => naga::Expression::Literal(naga::Literal::Bool(v)),
    //   PrimitiveShaderValue::Uint32(v) => naga::Expression::Literal(naga::Literal::U32(v)),
    //   PrimitiveShaderValue::Int32(v) => naga::Expression::Literal(naga::Literal::I32(v)),
    //   PrimitiveShaderValue::Float32(v) => naga::Expression::Literal(naga::Literal::F32(v)),
    //   PrimitiveShaderValue::Vec2Float32(v) => naga::Expression::Compose(naga::Literal::F32(v)),
    //   PrimitiveShaderValue::Vec3Float32(v) => todo!(),
    //   PrimitiveShaderValue::Vec4Float32(v) => todo!(),
    //   PrimitiveShaderValue::Vec2Uint32(v) => todo!(),
    //   PrimitiveShaderValue::Vec3Uint32(v) => todo!(),
    //   PrimitiveShaderValue::Vec4Uint32(v) => todo!(),
    //   PrimitiveShaderValue::Mat2Float32(v) => todo!(),
    //   PrimitiveShaderValue::Mat3Float32(v) => todo!(),
    //   PrimitiveShaderValue::Mat4Float32(v) => todo!(),
    // }
    // self.module.constants.append(value, Span::UNDEFINED)
    todo!()
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
    BinaryOperator::BitAnd => naga::BinaryOperator::And,
    BinaryOperator::BitOr => naga::BinaryOperator::InclusiveOr,
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

// enum ControlStructure{
//   If{
//     condition: naga::Handle<naga::Expression>,
//   }
// }

impl ShaderAPI for ShaderAPINagaImpl {
  fn register_ty(&mut self, ty: ShaderValueType) {
    self.register_ty_impl(ty);
  }

  fn define_module_input(&mut self, input: ShaderGraphInputNode) -> ShaderGraphNodeRawHandle {
    todo!()
  }

  fn define_frag_out(&mut self, idx: usize) -> ShaderGraphNodeRawHandle {
    todo!()
  }

  fn make_expression(&mut self, expr: ShaderGraphNodeExpr) -> ShaderGraphNodeRawHandle {
    let expr = match expr {
      ShaderGraphNodeExpr::FunctionCall { meta, parameters } => todo!(),
      ShaderGraphNodeExpr::TextureSampling {
        texture,
        sampler,
        position,
        index,
        level,
      } => naga::Expression::ImageSample {
        image: self.get_expression(texture),
        sampler: self.get_expression(sampler),
        gather: None,
        coordinate: self.get_expression(position),
        array_index: index.map(|index| self.get_expression(index)),
        offset: None,
        level: level
          .map(|level| naga::SampleLevel::Exact(self.get_expression(level)))
          .unwrap_or(naga::SampleLevel::Auto),
        depth_ref: None,
      },
      ShaderGraphNodeExpr::Swizzle { ty, source } => todo!(),
      ShaderGraphNodeExpr::Compose { target, parameters } => todo!(),
      ShaderGraphNodeExpr::MatShrink { source, dimension } => todo!(),
      ShaderGraphNodeExpr::Operator(op) => match op {
        OperatorNode::Unary { one, operator } => {
          let op = match operator {
            UnaryOperator::LogicalNot => naga::UnaryOperator::Not,
          };
          naga::Expression::Unary {
            op,
            expr: self.get_expression(one),
          }
        }
        OperatorNode::Binary {
          left,
          right,
          operator,
        } => {
          let left = self.get_expression(left);
          let right = self.get_expression(right);
          let op = map_binary_op(operator);
          naga::Expression::Binary { op, left, right }
        }
        OperatorNode::Index { array, entry } => naga::Expression::Access {
          base: self.get_expression(array),
          index: self.get_expression(entry),
        },
      },
      ShaderGraphNodeExpr::FieldGet {
        field_name,
        struct_node,
      } => {
        // let node_type
        // naga::Expression::AccessIndex {
        //   base: (),
        //   index: (),
        // }
        todo!()
      }
      ShaderGraphNodeExpr::StructConstruct { meta, fields } => {
        //   naga::Expression::Compose {
        //   ty: (),
        //   components: (),
        // }
        todo!()
      }
      ShaderGraphNodeExpr::Const(c) => {
        // let handle = self.module.constants.append(value, Span::UNDEFINED);
        todo!()
      }
    };

    self.make_expression_inner(expr)
  }

  fn make_var(&mut self, ty: ShaderValueType) -> ShaderGraphNodeRawHandle {
    let v = naga::LocalVariable {
      name: None,
      ty: self.register_ty_impl(ty),
      init: None,
    };
    let var = self
      .building_fn
      .last_mut()
      .unwrap()
      .1
      .local_variables
      .append(v, Span::UNDEFINED);

    self.make_expression_inner(naga::Expression::LocalVariable(var))
  }

  fn write(&mut self, source: ShaderGraphNodeRawHandle, target: ShaderGraphNodeRawHandle) {
    let st = naga::Statement::Store {
      pointer: self.get_expression(target),
      value: self.get_expression(source),
    };
    self.block.last_mut().unwrap().push(st, Span::UNDEFINED);
  }

  fn load(&mut self, source: ShaderGraphNodeRawHandle) -> ShaderGraphNodeRawHandle {
    let ex = naga::Expression::Load {
      pointer: self.get_expression(source),
    };
    self.make_expression_inner(ex)
  }

  fn push_scope(&mut self) {
    self.block.push(naga::Block::default())
  }

  fn pop_scope(&mut self) {
    let b = self.block.pop().unwrap();
    self
      .building_fn
      .last_mut()
      .unwrap()
      .1
      .body
      .push(naga::Statement::Block(b), Span::UNDEFINED);
  }

  fn push_if_scope(&mut self, condition: ShaderGraphNodeRawHandle) {
    self.block.push(Default::default());
    // let if_s = naga::Statement::If {
    //   condition: (),
    //   accept: (),
    //   reject: (),
    // };
    todo!()
  }

  fn discard(&mut self) {
    self
      .building_fn
      .last_mut()
      .unwrap()
      .1
      .body
      .push(naga::Statement::Kill, Span::UNDEFINED)
  }

  fn push_for_scope(&mut self, target: ShaderIterator) -> ForNodes {
    // self.block.push(naga::Block::default())
    todo!()
  }

  fn do_continue(&mut self, looper: ShaderGraphNodeRawHandle) {
    let st = naga::Statement::Continue;
    self.block.last_mut().unwrap().push(st, Span::UNDEFINED);
  }
  fn do_break(&mut self, looper: ShaderGraphNodeRawHandle) {
    let st = naga::Statement::Break;
    self.block.last_mut().unwrap().push(st, Span::UNDEFINED);
  }

  fn get_fn(&mut self, name: String) -> Option<ShaderFunctionMetaInfo> {
    self.fn_mapping.get(&name).map(|v| v.1.clone())
  }

  fn begin_define_fn(&mut self, name: String, return_ty: Option<ShaderValueType>) {
    if self.building_fn.iter().any(|f| f.0.eq(&name)) {
      panic!("recursive fn definition is not allowed")
    }

    self.fn_mapping.remove(&name);
    self.building_fn.push((name, Default::default(), 0));

    let (_, mut f, _) = self.building_fn.pop().unwrap();
    f.result = return_ty.map(|ty| naga::FunctionResult {
      ty: self.register_ty_impl(ty),
      binding: None,
    });
  }

  fn push_fn_parameter(&mut self, ty: ShaderValueType) -> ShaderGraphNodeRawHandle {
    let ty = self.register_ty_impl(ty);
    let last = self.building_fn.last_mut().unwrap();
    last.1.arguments.push(naga::FunctionArgument {
      name: None,
      ty,
      binding: None,
    });
    let expr = naga::Expression::FunctionArgument(last.2 as u32);
    last.2 += 1;
    self.make_expression_inner(expr)
  }

  fn do_return(&mut self, v: Option<ShaderGraphNodeRawHandle>) {
    todo!()
  }

  fn end_fn_define(&mut self) -> ShaderFunctionMetaInfo {
    let (name, f, _) = self.building_fn.pop().unwrap();
    let handle = self.module.functions.append(f, Span::UNDEFINED);
    self.fn_mapping.insert(name, (handle, todo!()));
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

    (wgsl, ENTRY_POINT_NAME.to_owned())
  }
}
