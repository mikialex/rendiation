use crate::*;

use Keyword as Kw;

impl SyntaxElement for FunctionDefine {
  fn parse<'a>(input: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    input.expect(Token::Keyword(Kw::Function))?;

    let name = parse_ident(input)?;
    input.expect(Token::Paren('('))?;
    let mut arguments = Vec::new();
    if !input.skip(Token::Paren(')')) {
      loop {
        let name = parse_ident(input)?;
        input.expect(Token::Separator(':'))?;
        let arg = TypeExpression::parse(input)?;
        arguments.push((name, arg));
        match input.next().token {
          Token::Paren(')') => break,
          Token::Separator(',') => (),
          other => return Err(ParseError::Unexpected(other, "argument list separator")),
        }
      }
    };
    let return_type = if input.skip(Token::Arrow) {
      Some(TypeExpression::parse(input)?)
    } else {
      None
    };

    let body = Block::parse(input)?;
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

fn check_primitive_ty(name: &str) -> Option<PrimitiveDataType> {
  match name {
    "vec2" => PrimitiveDataType::Vec2,
    "vec3" => PrimitiveDataType::Vec3,
    "vec4" => PrimitiveDataType::Vec4,
    "mat2x2" => PrimitiveDataType::Mat2,
    "mat3x3" => PrimitiveDataType::Mat3,
    "mat4x4" => PrimitiveDataType::Mat4,
    _ => return None,
  }
  .into()
}

fn check_value_ty(name: &str) -> Option<PrimitiveValueType> {
  match name {
    "f32" => PrimitiveValueType::Float32,
    "u32" => PrimitiveValueType::UnsignedInt32,
    "i32" => PrimitiveValueType::Int32,
    _ => return None,
  }
  .into()
}

impl SyntaxElement for PrimitiveType {
  fn parse<'a>(lexer: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let r = match lexer.next().token {
      Token::BuiltInType(name) => {
        let p_ty = if let Some(ty) = check_value_ty(name) {
          PrimitiveType::Scalar(ty)
        } else {
          let data_ty = check_primitive_ty(name).unwrap();

          lexer.expect(Token::Paren('<'))?;
          let value_ty = match lexer.next().token {
            Token::BuiltInType(name) => {
              check_value_ty(name).ok_or(ParseError::Any("unknown primitive value type"))?
            }
            _ => return Err(ParseError::Any("missing primitive value type")),
          };
          lexer.expect(Token::Paren('>'))?;
          PrimitiveType::Vector(PrimitiveVectorType { value_ty, data_ty })
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
  fn parse<'a>(input: &mut Lexer<'a>) -> Result<Self, ParseError<'a>> {
    let r = match input.next().token {
      Token::Keyword(kw) => match kw {
        Keyword::Case => todo!(),
        Keyword::Default => {
          input.skip(Token::Separator(':'));
          let (statements, fallthrough) = parse_case_compound_statement(input)?;
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

fn parse_case_compound_statement<'a>(
  lexer: &mut Lexer<'a>,
) -> Result<(Vec<Statement>, bool), ParseError<'a>> {
  let mut statements = Vec::new();
  lexer.expect(Token::Paren('{'))?;
  let mut fallthrough = false;
  while lexer.peek().token != Token::Paren('}')
    || lexer.peek().token != Token::Keyword(Kw::FallThrough)
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

          while lexer.peek().token == Token::Keyword(Kw::ElseIf) {
            lexer.expect(Token::Keyword(Kw::ElseIf))?;
            elses.push(IfElse {
              condition: Expression::parse(lexer)?,
              accept: Block::parse(lexer)?,
            });
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
          let init = parse_expression_like_statement(lexer)?;
          let test = parse_expression_like_statement(lexer)?;
          let update = Expression::parse(lexer)?;
          let body = Block::parse(lexer)?;
          Statement::For(crate::ast::For {
            init: Box::new(init),
            test: Box::new(test),
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

pub fn parse_expression_like_statement<'a>(
  lexer: &mut Lexer<'a>,
) -> Result<Statement, ParseError<'a>> {
  let mut lex = lexer.clone();
  let mut lex2 = lexer.clone();
  let mut has_assign = false;
  loop {
    match lex.next().token {
      Token::Operation('=') => has_assign = true,
      Token::Separator(';') => break,
      _ => {}
    }
  }

  let r = match lexer.next().token {
    Token::Keyword(Kw::Declare(declare_ty)) => {
      let name = parse_ident(lexer)?;
      let ty = if lexer.skip(Token::Separator(':')) {
        TypeExpression::parse(lexer)?.into()
      } else {
        None
      };

      let exp = if let Token::Operation('=') = lexer.peek().token {
        lexer.expect(Token::Operation('='))?;
        Expression::parse(lexer)?.into()
      } else {
        None
      };

      lexer.expect(Token::Separator(';'))?;

      Statement::Declare {
        declare_ty,
        ty,
        name,
        init: exp,
      }
    }
    Token::Word(name) => {
      if has_assign {
        let name = Ident::from(name);
        lexer.expect(Token::Operation('='))?;
        let exp = Expression::parse(lexer)?;
        lexer.expect(Token::Separator(';'))?;
        Statement::Assignment { name, value: exp }
      } else {
        let exp = Expression::parse(&mut lex2)?;
        *lexer = lex2;
        lexer.expect(Token::Separator(';'))?;
        Statement::Expression(exp)
      }
    }
    Token::Separator(';') => {
      let _ = lexer.next();
      Statement::Empty
    }
    _ => {
      let exp = Expression::parse(lexer)?;
      lexer.expect(Token::Separator(';'))?;
      Statement::Expression(exp)
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
      Token::LogicalOperation('=') => Some(BinaryOperator::Equal),
      Token::LogicalOperation('!') => Some(BinaryOperator::NotEqual),
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
                parse_exp_with_postfix,
              )
            },
          )
        },
      )
    },
  )
}

// EXP_WITH_POSTFIX
pub fn parse_exp_with_postfix<'a>(input: &mut Lexer<'a>) -> Result<Expression, ParseError<'a>> {
  let mut result = parse_single_expression(input)?;
  loop {
    result = match input.peek().token {
      Token::Paren('[') => {
        let _ = input.next();
        let index = parse_single_expression(input)?;
        input.expect(Token::Paren(']'))?;
        Expression::ArrayAccess {
          array: Box::new(result),
          index: Box::new(index),
        }
      }
      Token::Separator('.') => {
        let _ = input.next();
        match input.next().token {
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
pub fn parse_single_expression<'a>(input: &mut Lexer<'a>) -> Result<Expression, ParseError<'a>> {
  let mut backup = input.clone();
  let r = match input.next().token {
    Token::Number { .. } => Expression::PrimitiveConst(PrimitiveConstValue::Numeric(
      NumericTypeConstValue::Float(1.), // todo
    )),
    Token::Bool(v) => Expression::PrimitiveConst(PrimitiveConstValue::Bool(v)),
    Token::Operation('-') => {
      let inner = Expression::parse(input)?;
      let inner = Box::new(inner);
      Expression::UnaryOperator {
        op: UnaryOperator::Neg,
        expr: inner,
      }
    }
    Token::Operation('!') => {
      let inner = Expression::parse(input)?;
      let inner = Box::new(inner);
      Expression::UnaryOperator {
        op: UnaryOperator::Not,
        expr: inner,
      }
    }
    Token::Paren('(') => {
      let inner = Expression::parse(input)?;
      input.expect(Token::Paren(')'))?;
      inner
    }
    Token::BuiltInType(_) => {
      let ty = PrimitiveType::parse(&mut backup)?;
      *input = backup;
      Expression::PrimitiveConstruct {
        ty,
        arguments: parse_function_parameters(input)?,
      }
    }
    Token::Word(name) => {
      if let Token::Paren('(') = input.peek().token {
        Expression::FunctionCall(FunctionCall {
          name: Ident::from(name),
          arguments: parse_function_parameters(input)?,
        })
      } else {
        Expression::Ident(Ident {
          name: name.to_owned(),
        })
      }
    }
    _ => return Err(ParseError::Any("failed in parse single expression")),
  };
  Ok(r)
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
  input: &mut Lexer<'a>,
) -> Result<Vec<Expression>, ParseError<'a>> {
  input.expect(Token::Paren('('))?;
  let mut arguments = Vec::new();
  // if skipped means empty argument
  if !input.skip(Token::Paren(')')) {
    loop {
      let arg = Expression::parse(input)?;
      arguments.push(arg);
      match input.next().token {
        Token::Paren(')') => break,
        Token::Separator(',') => (),
        other => return Err(ParseError::Unexpected(other, "argument list separator")),
      }
    }
  }
  Ok(arguments)
}
