type Span = std::ops::Range<usize>;

// pub struct ASTContext {
//   expression: Arena<Expression>,
// }

// pub enum Expression {
//   /// Vector swizzle.
//   Swizzle {
//     size: VectorSize,
//     vector: Handle<Expression>,
//     pattern: [SwizzleComponent; 4],
//   },
//   /// Composite expression.
//   Compose {
//     ty: Handle<Type>,
//     components: Vec<Handle<Expression>>,
//   },
// }

// pub struct StructDefinition {
//   fields: Vec<StructField>,
// }

// pub struct StructField {
//   name: Span,
//   ty: (),
// }
