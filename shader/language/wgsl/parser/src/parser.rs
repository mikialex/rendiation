use Keyword as Kw;

use crate::*;

impl SyntaxElement for FunctionDefine {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    lexer.expect(Token::Keyword(Kw::Function))?;

    let name = parse_ident(lexer)?;
    lexer.expect(Token::Paren('('))?;
    let mut arguments = Vec::new();
    if !lexer.skip(Token::Paren(')')) {
      loop {
        let name = parse_ident(lexer)?;
        lexer.expect(Token::Separator(':'))?;
        let arg = TypeExpression::parse(lexer)?;
        arguments.push((name, arg));
        match lexer.next().token {
          Token::Paren(')') => break,
          Token::Separator(',') => {
            // the last ',' is optional
            if lexer.peek().token == Token::Paren(')') {
              let _ = lexer.next();
              break;
            }
          }
          other => return Err(ParseError::Unexpected(other, "argument list separator")),
        }
      }
    };
    let return_type = if lexer.skip(Token::Arrow) {
      Some(TypeExpression::parse(lexer)?)
    } else {
      None
    };

    let body = Block::parse(lexer)?;
    Ok(FunctionDefine {
      name,
      arguments,
      return_type,
      body,
    })
  }
}

fn parse_ident<'a>(lexer: &mut Lexer<'a>) -> Result<Ident, ParseError<'a>> {
  let r = match lexer.next().token {
    Token::Word(name) => Ident::from(name),
    _ => return Err(ParseError::Any("cant parse ident")),
  };
  Ok(r)
}

fn check_vec_ty(name: &str) -> Option<PrimitiveVecDataType> {
  match name {
    "vec2" => PrimitiveVecDataType::Vec2,
    "vec3" => PrimitiveVecDataType::Vec3,
    "vec4" => PrimitiveVecDataType::Vec4,
    "mat2x2" => PrimitiveVecDataType::Mat2,
    "mat3x3" => PrimitiveVecDataType::Mat3,
    "mat4x4" => PrimitiveVecDataType::Mat4,
    _ => return None,
  }
  .into()
}

fn check_texture_ty(name: &str) -> Option<TextureContainerType> {
  match name {
    "texture_1d" => TextureContainerType::D1,
    "texture_2d" => TextureContainerType::D2,
    "texture_2d_array" => TextureContainerType::D2Array,
    "texture_3d" => TextureContainerType::D3,
    "texture_cube" => TextureContainerType::Cube,
    "texture_cube_array" => TextureContainerType::CubeArray,
    _ => return None,
  }
  .into()
}

fn check_depth_texture_ty(name: &str) -> Option<DepthTextureContainerType> {
  match name {
    "texture_depth_2d" => DepthTextureContainerType::D2,
    "texture_depth_2d_array" => DepthTextureContainerType::D2Array,
    "texture_depth_cube" => DepthTextureContainerType::Cube,
    "texture_depth_cube_array" => DepthTextureContainerType::CubeArray,
    _ => return None,
  }
  .into()
}

fn check_value_ty(name: &str) -> Option<PrimitiveValueType> {
  match name {
    "f32" => PrimitiveValueType::Float32,
    "u32" => PrimitiveValueType::UnsignedInt32,
    "i32" => PrimitiveValueType::Int32,
    "bool" => PrimitiveValueType::Bool,
    _ => return None,
  }
  .into()
}

impl SyntaxElement for PrimitiveValueType {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    match lexer.next().token {
      Token::BuiltInType(name) => {
        check_value_ty(name).ok_or(ParseError::Any("unknown primitive value type"))
      }
      _ => Err(ParseError::Any("missing primitive value type")),
    }
  }
}

impl SyntaxElement for PrimitiveType {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let r = match lexer.next().token {
      Token::BuiltInType(name) => {
        let p_ty = if let Some(ty) = check_value_ty(name) {
          PrimitiveType::Scalar(ty)
        } else if let Some(vec_ty) = check_vec_ty(name) {
          lexer.expect(Token::Paren('<'))?;
          let value_ty = PrimitiveValueType::parse(lexer)?;
          lexer.expect(Token::Paren('>'))?;
          PrimitiveType::Vector(PrimitiveVectorType { value_ty, vec_ty })
        } else if let Some(container_ty) = check_texture_ty(name) {
          lexer.expect(Token::Paren('<'))?;
          let value_ty = PrimitiveValueType::parse(lexer)?;
          lexer.expect(Token::Paren('>'))?;
          PrimitiveType::Texture(TextureType {
            value_ty,
            container_ty,
          })
        } else if let Some(container_ty) = check_depth_texture_ty(name) {
          PrimitiveType::DepthTexture(container_ty)
        } else if name == "sampler" {
          PrimitiveType::Sampler
        } else if name == "sampler_comparison" {
          PrimitiveType::DepthSampler
        } else {
          return Err(ParseError::Any("unexpected builtin type"));
        };
        p_ty
      }
      _ => return Err(ParseError::Any("cant parse primitive type")),
    };
    Ok(r)
  }
}

impl SyntaxElement for TypeExpression {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let r = match lexer.peek().token {
      Token::Word(name) => {
        _ = lexer.next();
        TypeExpression::Struct(Ident::from(name))
      }
      Token::BuiltInType(_) => TypeExpression::Primitive(PrimitiveType::parse(lexer)?),
      Token::Array => {
        _ = lexer.next();
        lexer.expect(Token::Paren('<'))?;
        let array_ty = Self::parse(lexer)?;
        lexer.expect(Token::Separator(','))?;
        let size = match lexer.next().token {
          Token::Number { value, .. } => {
            if let Ok(size) = value.parse::<u32>() {
              size
            } else {
              return Err(ParseError::Any("expect array length"));
            }
          }
          _ => return Err(ParseError::Any("expect array length")),
        };
        lexer.expect(Token::Paren('>'))?;
        TypeExpression::FixedArray((Box::new(array_ty), size as usize))
      }
      _ => return Err(ParseError::Any("cant parse type_expression")),
    };
    Ok(r)
  }
}

impl SyntaxElement for Block {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let mut block = Block {
      statements: Vec::new(),
    };
    lexer.expect(Token::Paren('{'))?;
    while lexer.peek().token != Token::Paren('}') {
      block.statements.push(Statement::parse(lexer)?);
    }
    lexer.expect(Token::Paren('}'))?;
    Ok(block)
  }
}

impl SyntaxElement for Switch {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    lexer.expect(Token::Keyword(Kw::Switch))?;
    let target = Expression::parse(lexer)?;
    lexer.expect(Token::Paren('{'))?;
    let mut cases = Vec::new();
    while lexer.peek().token != Token::Paren('}') {
      cases.push(SwitchBody::parse(lexer)?);
    }
    lexer.expect(Token::Paren('}'))?;
    Ok(Self { target, cases })
  }
}

impl SyntaxElement for SwitchBody {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let r = match lexer.next().token {
      Token::Keyword(kw) => match kw {
        Keyword::Case => {
          let selectors = parse_case_selectors(lexer)?;
          lexer.skip(Token::Separator(':'));
          let (statements, fallthrough) = parse_case_compound_statement(lexer)?;
          Self {
            case: CaseType::Const(selectors),
            statements,
            fallthrough,
          }
        }
        Keyword::Default => {
          lexer.skip(Token::Separator(':'));
          let (statements, fallthrough) = parse_case_compound_statement(lexer)?;
          Self {
            case: CaseType::Default,
            statements,
            fallthrough,
          }
        }
        _ => {
          return Err(ParseError::Any(
            "failed to parse switch body, expect case or default",
          ))
        }
      },
      _ => {
        return Err(ParseError::Any(
          "failed to parse switch body, expect case or default",
        ))
      }
    };
    Ok(r)
  }
}

fn parse_case_selectors<'a>(lexer: &mut Lexer<'a>) -> Result<Vec<Expression>, ParseError<'a>> {
  let mut re = Vec::new();
  loop {
    re.push(Expression::parse(lexer)?);
    match lexer.peek().token {
      Token::Separator(',') => continue,
      Token::Separator(':') => break,
      Token::Separator('{') => break,
      _ => return Err(ParseError::Any("expect colon or comma or selection block")),
    }
  }
  Ok(re)
}

fn parse_case_compound_statement<'a>(
  lexer: &mut Lexer<'a>,
) -> Result<(Vec<Statement>, bool), ParseError<'a>> {
  let mut statements = Vec::new();
  lexer.expect(Token::Paren('{'))?;
  let mut fallthrough = false;
  while lexer.peek().token != Token::Paren('}')
    && lexer.peek().token != Token::Keyword(Kw::FallThrough)
  {
    if lexer.skip(Token::Keyword(Kw::FallThrough)) {
      fallthrough = true;
      lexer.expect(Token::Separator(';'))?;
    } else {
      if fallthrough {
        return Err(ParseError::Any(
          "fallthrough should be last statement in switch body",
        ));
      }
      statements.push(Statement::parse(lexer)?);
    }
  }
  lexer.expect(Token::Paren('}'))?;
  Ok((statements, fallthrough))
}

impl SyntaxElement for Statement {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let r = match lexer.peek().token {
      Token::Keyword(keyword) => match keyword {
        Kw::Declare(_) => parse_expression_like_statement(lexer)?,
        Kw::Return => {
          let _ = lexer.next();
          let value = if lexer.peek().token == Token::Separator(';') {
            None
          } else {
            Some(Expression::parse(lexer)?)
          };
          lexer.expect(Token::Separator(';'))?;
          Statement::Return { value }
        }
        Kw::Loop => {
          let _ = lexer.next();
          lexer.expect(Token::Separator(';'))?;
          // Statement::Discard
          todo!()
        }
        Kw::Switch => Statement::Switch(Switch::parse(lexer)?),
        Kw::Discard => {
          let _ = lexer.next();
          lexer.expect(Token::Separator(';'))?;
          Statement::Discard
        }
        Kw::Break => {
          let _ = lexer.next();
          lexer.expect(Token::Separator(';'))?;
          Statement::Break
        }
        Kw::Continue => {
          let _ = lexer.next();
          lexer.expect(Token::Separator(';'))?;
          Statement::Continue
        }
        Kw::If => {
          let _ = lexer.next();
          let condition = Expression::parse(lexer)?;
          let accept = Block::parse(lexer)?;
          let mut elses = Vec::new();

          while lexer.peek().token == Token::Keyword(Kw::Else) {
            lexer.expect(Token::Keyword(Kw::Else))?;
            if lexer.peek().token == Token::Keyword(Kw::If) {
              lexer.expect(Token::Keyword(Kw::If))?;
              elses.push(IfElse {
                condition: Expression::parse(lexer)?,
                accept: Block::parse(lexer)?,
              });
            } else {
              break;
            }
          }

          let reject = if lexer.skip(Token::Keyword(Kw::Else)) {
            Some(Block::parse(lexer)?)
          } else {
            None
          };

          lexer.skip(Token::Separator(';'));
          Statement::If(If {
            condition,
            accept,
            elses,
            reject,
          })
        }
        Kw::For => {
          let _ = lexer.next();

          lexer.expect(Token::Paren('('))?;
          let init = Option::<ForInit>::parse(lexer)?;
          let test = match parse_expression_like_statement(lexer)? {
            Statement::Expression(e) => Some(e),
            _ => None,
          };
          let update = Option::<ForUpdate>::parse(lexer)?;
          lexer.expect(Token::Paren(')'))?;

          let body = Block::parse(lexer)?;

          Statement::For(For {
            init,
            test,
            update,
            body,
          })
        }
        Kw::While => {
          let _ = lexer.next();
          Statement::While(While {
            condition: Expression::parse(lexer)?,
            body: Block::parse(lexer)?,
          })
        }
        _ => return Err(ParseError::Any("cant parse statement")),
      },
      Token::Paren('{') => Statement::Block(Block::parse(lexer)?),
      _ => parse_expression_like_statement(lexer)?,
    };
    Ok(r)
  }
}

impl SyntaxElement for Option<ForInit> {
  #[allow(clippy::collapsible_match)]
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let statement = parse_expression_like_statement(lexer)?;
    let r = match statement {
      Statement::Declare(d) => ForInit::Declare(d),
      Statement::Empty => return Ok(None),
      Statement::Assignment(a) => ForInit::Assignment(a),
      Statement::Increment(s) => ForInit::Increment(s),
      Statement::Decrement(s) => ForInit::Decrement(s),
      Statement::Expression(exp) => match exp {
        Expression::FunctionCall(call) => ForInit::Call(call),
        _ => return Err(ParseError::Any("invalid for init")),
      },
      _ => return Err(ParseError::Any("invalid for init")),
    };
    Ok(Some(r))
  }
}

impl SyntaxElement for Option<ForUpdate> {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    // because the ForUpdate statement not followed by ';', so we can not reuse the statement parser
    // we simply try every choice here
    match lexer.peek().token {
      Token::Paren(')') => Ok(None),
      _ => {
        let r = Assignment::try_parse(lexer)
          .map(ForUpdate::Assignment)
          .or_else(|_| Increment::try_parse(lexer).map(ForUpdate::Increment))
          .or_else(|_| Decrement::try_parse(lexer).map(ForUpdate::Decrement))
          .or_else(|_| FunctionCall::try_parse(lexer).map(ForUpdate::Call))?;
        Ok(Some(r))
      }
    }
  }
}

impl SyntaxElement for Increment {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let lhs = LhsExpression::parse(lexer)?;
    lexer.expect(Token::Increment)?;
    Ok(Self(lhs))
  }
}

impl SyntaxElement for Decrement {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let lhs = LhsExpression::parse(lexer)?;
    lexer.expect(Token::Decrement)?;
    Ok(Self(lhs))
  }
}

impl SyntaxElement for VariableStatement {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    match lexer.next().token {
      Token::Keyword(Kw::Declare(declare_ty)) => {
        let name = parse_ident(lexer)?;
        let ty = if lexer.skip(Token::Separator(':')) {
          TypeExpression::parse(lexer)?.into()
        } else {
          None
        };

        let exp = if let Token::Assign = lexer.peek().token {
          lexer.expect(Token::Assign)?;
          Expression::parse(lexer)?.into()
        } else {
          None
        };
        Ok(VariableStatement {
          declare_ty,
          ty,
          name,
          init: exp,
        })
      }
      _ => Err(ParseError::Any("expect let or var")),
    }
  }
}

impl SyntaxElement for Assignment {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let lhs = LhsExpression::parse(lexer)?;
    let assign_op = match lexer.next().token {
      Token::Assign => None,
      Token::CompoundAssign(ass) => Some(ass),
      _ => return Err(ParseError::Any("expect assign or assign op")),
    };
    let value = Expression::parse(lexer)?;
    Ok(Assignment {
      lhs,
      assign_op,
      value,
    })
  }
}

pub fn parse_expression_like_statement<'a>(
  lexer: &mut Lexer<'a>,
) -> Result<Statement, ParseError<'a>> {
  let mut checker = lexer.clone();
  let mut has_assign = false;
  let mut is_increment = false;
  let mut is_decrement = false;
  loop {
    match checker.next().token {
      Token::Assign => has_assign = true,
      Token::CompoundAssign(_) => has_assign = true,
      Token::Increment => is_increment = true,
      Token::Decrement => is_decrement = true,
      Token::Separator(';') => break,
      _ => {}
    }
  }

  let r = match lexer.peek().token {
    Token::Keyword(Kw::Declare(_)) => {
      let var = VariableStatement::parse(lexer)?;
      lexer.expect(Token::Separator(';'))?;
      Statement::Declare(var)
    }
    Token::Separator(';') => {
      let _ = lexer.next();
      Statement::Empty
    }
    _ => {
      if has_assign {
        let ass = Assignment::parse(lexer)?;
        lexer.expect(Token::Separator(';'))?;
        Statement::Assignment(ass)
      } else if is_increment {
        let i = Increment::parse(lexer)?;
        lexer.expect(Token::Separator(';'))?;
        Statement::Increment(i)
      } else if is_decrement {
        let i = Decrement::parse(lexer)?;
        lexer.expect(Token::Separator(';'))?;
        Statement::Decrement(i)
      } else {
        let exp = Expression::parse(lexer)?;
        lexer.expect(Token::Separator(';'))?;
        Statement::Expression(exp)
      }
    }
  };

  Ok(r)
}

// EXP

impl SyntaxElement for Expression {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    parse_exp_with_binary_operators(lexer)
  }
}

pub fn parse_exp_with_binary_operators<'a>(
  lexer: &mut Lexer<'a>,
) -> Result<Expression, ParseError<'a>> {
  parse_binary_op_left(
    lexer,
    |token| match token {
      Token::LogicalOperation('|') => Some(BinaryOperator::LogicalOr),
      _ => None,
    },
    // logical_and_expression
    |lexer| {
      parse_binary_op_left(
        lexer,
        |token| match token {
          Token::LogicalOperation('&') => Some(BinaryOperator::LogicalAnd),
          _ => None,
        },
        // inclusive_or_expression
        |lexer| {
          parse_binary_op_left(
            lexer,
            |token| match token {
              Token::Operation('|') => Some(BinaryOperator::Or),
              _ => None,
            },
            // exclusive_or_expression
            |lexer| {
              parse_binary_op_left(
                lexer,
                |token| match token {
                  Token::Operation('^') => Some(BinaryOperator::Xor),
                  _ => None,
                },
                // and_expression
                |lexer| {
                  parse_binary_op_left(
                    lexer,
                    |token| match token {
                      Token::Operation('&') => Some(BinaryOperator::And),
                      _ => None,
                    },
                    parse_exp_with_binary_operators_no_logic_no_bit,
                  )
                },
              )
            },
          )
        },
      )
    },
  )
}

pub fn parse_exp_with_binary_operators_no_logic_no_bit<'a>(
  lexer: &mut Lexer<'a>,
) -> Result<Expression, ParseError<'a>> {
  // equality_expression
  parse_binary_op_left(
    lexer,
    |token| match token {
      Token::Equals => Some(BinaryOperator::Equal),
      Token::NotEquals => Some(BinaryOperator::NotEqual),
      _ => None,
    },
    // relational_expression
    |lexer| {
      parse_binary_op_left(
        lexer,
        |token| match token {
          Token::Paren('<') => Some(BinaryOperator::Less),
          Token::Paren('>') => Some(BinaryOperator::Greater),
          Token::LogicalOperation('<') => Some(BinaryOperator::LessEqual),
          Token::LogicalOperation('>') => Some(BinaryOperator::GreaterEqual),
          _ => None,
        },
        |lexer| {
          // additive_expression
          parse_binary_op_left(
            lexer,
            |token| match token {
              Token::Operation('+') => Some(BinaryOperator::Add),
              Token::Operation('-') => Some(BinaryOperator::Sub),
              _ => None,
            },
            // multiplicative_expression
            |lexer| {
              parse_binary_op_left(
                lexer,
                |token| match token {
                  Token::Operation('*') => Some(BinaryOperator::Mul),
                  Token::Operation('/') => Some(BinaryOperator::Div),
                  Token::Operation('%') => Some(BinaryOperator::Mod),
                  _ => None,
                },
                parse_singular_expression,
              )
            },
          )
        },
      )
    },
  )
}

// EXP_WITH_POSTFIX
pub fn parse_singular_expression<'a>(lexer: &mut Lexer<'a>) -> Result<Expression, ParseError<'a>> {
  let mut result = parse_primary_expression(lexer)?;
  loop {
    result = match lexer.peek().token {
      Token::Paren('[') => {
        let _ = lexer.next();
        let index = parse_primary_expression(lexer)?;
        lexer.expect(Token::Paren(']'))?;
        Expression::ArrayAccess {
          array: Box::new(result),
          index: Box::new(index),
        }
      }
      Token::Separator('.') => {
        let _ = lexer.next();
        match lexer.next().token {
          Token::Word(name) => Expression::ItemAccess {
            from: Box::new(result),
            to: Ident::from(name),
          },
          _ => return Err(ParseError::Any("only ident can dot with")),
        }
      }
      _ => break,
    };
  }

  Ok(result)
}

// EXP_SINGLE
pub fn parse_primary_expression<'a>(lexer: &mut Lexer<'a>) -> Result<Expression, ParseError<'a>> {
  let mut backup = lexer.clone();
  let r = match lexer.next().token {
    Token::Number { value, ty, .. } => {
      Expression::PrimitiveConst(PrimitiveConstValue::Numeric(match ty {
        'u' => NumericTypeConstValue::UnsignedInt(value.parse::<u32>().unwrap()),
        'i' => NumericTypeConstValue::Int(value.parse::<i32>().unwrap()),
        'f' => NumericTypeConstValue::Float(value.parse::<f32>().unwrap()),
        _ => return Err(ParseError::Any("unknown number ty")),
      }))
    }
    Token::Bool(v) => Expression::PrimitiveConst(PrimitiveConstValue::Bool(v)),
    Token::Operation('-') => {
      let inner = Expression::parse(lexer)?;
      let inner = Box::new(inner);
      Expression::UnaryOperator {
        op: UnaryOperator::Neg,
        expr: inner,
      }
    }
    Token::Operation('!') => {
      let inner = Expression::parse(lexer)?;
      let inner = Box::new(inner);
      Expression::UnaryOperator {
        op: UnaryOperator::Not,
        expr: inner,
      }
    }
    Token::Paren('(') => {
      let inner = Expression::parse(lexer)?;
      lexer.expect(Token::Paren(')'))?;
      inner
    }
    Token::BuiltInType(_) => {
      let ty = PrimitiveType::parse(&mut backup)?;
      *lexer = backup;
      Expression::PrimitiveConstruct {
        ty,
        arguments: parse_function_parameters(lexer)?,
      }
    }
    Token::Word(name) => {
      if let Token::Paren('(') = lexer.peek().token {
        let call = FunctionCall::parse(&mut backup)?;
        *lexer = backup;
        Expression::FunctionCall(call)
      } else {
        Expression::Ident(Ident {
          name: name.to_owned(),
        })
      }
    }
    _ => panic!(), // _ => return Err(ParseError::Any("failed in parse single expression")),
  };
  Ok(r)
}

impl SyntaxElement for FunctionCall {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    match lexer.next().token {
      // todo expect word
      Token::Word(name) => Ok(FunctionCall {
        name: Ident::from(name),
        arguments: parse_function_parameters(lexer)?,
      }),
      _ => Err(ParseError::Any("expect function name")),
    }
  }
}

fn parse_binary_op_left<'a>(
  lexer: &mut Lexer<'a>,
  separator: impl Fn(Token<'a>) -> Option<BinaryOperator>,
  parser: impl Fn(&mut Lexer<'a>) -> Result<Expression, ParseError<'a>>,
) -> Result<Expression, ParseError<'a>> {
  parse_binary_like_left(
    lexer,
    |tk| separator(tk).is_some(),
    &parser,
    &parser,
    |left, tk, right| Expression::BinaryOperator {
      op: separator(tk).unwrap(), // this unwrap is safe
      left: Box::new(left),
      right: Box::new(right),
    },
  )
}

fn parse_binary_like_left<'a, L, R>(
  lexer: &mut Lexer<'a>,
  separator: impl Fn(Token<'a>) -> bool,
  left_parser: &impl Fn(&mut Lexer<'a>) -> Result<L, ParseError<'a>>,
  right_parser: &impl Fn(&mut Lexer<'a>) -> Result<R, ParseError<'a>>,
  assemble: impl Fn(L, Token<'a>, R) -> L,
) -> Result<L, ParseError<'a>> {
  let mut result = left_parser(lexer)?;
  while separator(lexer.peek().token) {
    let token = lexer.next().token;
    let right = right_parser(lexer)?;
    result = assemble(result, token, right);
  }
  Ok(result)
}

#[allow(unused)]
fn parse_binary_like_right<'a, L, R>(
  lexer: &mut Lexer<'a>,
  separator: &impl Fn(Token<'a>) -> bool,
  left_parser: &impl Fn(&mut Lexer<'a>) -> Result<L, ParseError<'a>>,
  right_parser: &impl Fn(&mut Lexer<'a>) -> Result<R, ParseError<'a>>,
  assemble: &impl Fn(L, Token<'a>, R) -> R,
) -> Result<R, ParseError<'a>> {
  let mut backup = lexer.clone();
  let left = left_parser(lexer);
  if let Ok(left) = left {
    loop {
      if separator(lexer.peek().token) {
        let token = lexer.next().token;
        let right = parse_binary_like_right(lexer, separator, left_parser, right_parser, assemble)?;
        return Ok(assemble(left, token, right));
      }
    }
    right_parser(lexer)
  } else {
    let r = right_parser(&mut backup);
    *lexer = backup;
    r
  }
}

pub fn parse_function_parameters<'a>(
  lexer: &mut Lexer<'a>,
) -> Result<Vec<Expression>, ParseError<'a>> {
  lexer.expect(Token::Paren('('))?;
  let mut arguments = Vec::new();
  // if skipped means empty argument
  if !lexer.skip(Token::Paren(')')) {
    loop {
      let arg = Expression::parse(lexer)?;
      arguments.push(arg);
      match lexer.next().token {
        Token::Paren(')') => break,
        Token::Separator(',') => {
          // the last ',' is optional
          if lexer.peek().token == Token::Paren(')') {
            let _ = lexer.next();
            break;
          }
        }
        other => return Err(ParseError::Unexpected(other, "argument list separator")),
      }
    }
  }
  Ok(arguments)
}

impl SyntaxElement for LhsExpression {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let content = LhsExpressionCore::parse(lexer)?;
    let postfix = Vec::<PostFixExpression>::parse(lexer)?;
    Ok(Self { content, postfix })
  }
}

impl SyntaxElement for LhsExpressionCore {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let r = match lexer.next().token {
      Token::Word(name) => Self::Ident(Ident::from(name)),
      Token::Paren('(') => {
        let r = Self::parse(lexer)?;
        lexer.expect(Token::Paren(')'))?;
        return Ok(r);
      }
      Token::Operation('*') => {
        let r = LhsExpression::parse(lexer)?;
        Self::Deref(Box::new(r))
      }
      Token::Operation('&') => {
        let r = LhsExpression::parse(lexer)?;
        Self::Ref(Box::new(r))
      }
      _ => return Err(ParseError::Any("expect ident, deref or ref operator")),
    };
    Ok(r)
  }
}

impl SyntaxElement for Vec<PostFixExpression> {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let mut r = Vec::new();
    loop {
      match lexer.peek().token {
        Token::Separator('[') => {
          let _ = lexer.next();
          let index = Expression::parse(lexer)?;
          r.push(PostFixExpression::ArrayAccess {
            index: Box::new(index),
          })
        }
        Token::Separator('.') => {
          let _ = lexer.next();
          r.push(PostFixExpression::FieldAccess {
            field: parse_ident(lexer)?,
          })
        }
        _ => break,
      };
    }

    Ok(r)
  }
}
