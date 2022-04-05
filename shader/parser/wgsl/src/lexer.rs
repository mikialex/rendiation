use std::ops::Range;

use crate::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Keyword {
  If,
  ElseIf,
  Else,
  For,
  While,
  Return,
  Break,
  Continue,
  Declare(DeclarationType),
  Function,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Token<'a> {
  Separator(char),
  DoubleColon,
  Paren(char),
  DoubleParen(char),
  Number {
    value: &'a str,
    ty: char,
    width: &'a str,
  },
  Bool(bool),
  String(&'a str),
  Word(&'a str),
  Keyword(Keyword),
  BuiltInType(&'a str),
  Operation(char),
  LogicalOperation(char),
  ShiftOperation(char),
  Arrow,
  Unknown(char),
  UnterminatedString,
  Trivia,
  End,
}

#[derive(Debug)]
pub struct TokenSpan<'a> {
  pub token: Token<'a>,
  pub range: SourceRange,
}

#[derive(Debug)]
pub struct SourceRange {
  pub column_range: Range<usize>,
  pub row_start: usize,
  pub row_end: usize,
}

#[derive(Clone, Debug)]
struct ReadCursor {
  column: usize,
  row: usize,
}

#[derive(Clone, Debug)]
pub struct Lexer<'a> {
  input: &'a str,
  cursor: ReadCursor,
  pub parsing_type: bool,
}

impl<'a> Lexer<'a> {
  pub fn new(input: &'a str) -> Self {
    Lexer {
      input,
      cursor: ReadCursor { column: 0, row: 0 },
      parsing_type: false,
    }
  }

  fn peek_token_and_rest(&self) -> (TokenSpan<'a>, &'a str) {
    let mut cloned = self.clone();
    let token = cloned.next();
    let rest = cloned.input;
    (token, rest)
  }

  #[must_use]
  pub fn next(&mut self) -> TokenSpan<'a> {
    loop {
      let token = self.consume_token();
      match token.token {
        Token::Trivia => continue,
        _ => return token,
      }
    }
  }

  #[must_use]
  pub fn peek(&self) -> TokenSpan<'a> {
    let (token, _) = self.peek_token_and_rest();
    token
  }

  pub fn expect(&mut self, expected: Token<'a>) -> Result<(), ParseError<'a>> {
    let next = self.next();
    if next.token == expected {
      Ok(())
    } else {
      let description = match expected {
        Token::Separator(_) => "separator",
        Token::DoubleColon => "::",
        Token::Paren(_) => "paren",
        Token::DoubleParen(_) => "double paren",
        Token::Number { .. } => "number",
        Token::String(string) => string,
        Token::Word(word) => word,
        Token::Keyword(_) => "Keyword",
        Token::Operation(_) => "operation",
        Token::LogicalOperation(_) => "logical op",
        Token::ShiftOperation(_) => "shift op",
        Token::Arrow => "->",
        Token::Unknown(_) => "unknown",
        Token::UnterminatedString => "string",
        Token::Trivia => "trivia",
        Token::Bool(_) => "boolean",
        Token::End => "",
        Token::BuiltInType(_) => "builtin_type",
      };
      panic!("ddd");
      Err(ParseError::Unexpected(next.token, description))
    }
  }

  pub fn skip(&mut self, what: Token<'_>) -> bool {
    let (peeked_token, rest) = self.peek_token_and_rest();
    if peeked_token.token == what {
      self.input = rest;
      true
    } else {
      false
    }
  }
}

impl<'a> Lexer<'a> {
  fn consume_token(&mut self) -> TokenSpan<'a> {
    let mut input = self.input;
    let start_cursor = self.cursor.clone();

    let mut chars = input.chars();
    let cur = match chars.next() {
      Some(c) => c,
      None => {
        return TokenSpan {
          token: Token::End,
          range: SourceRange {
            column_range: start_cursor.row..self.cursor.column,
            row_start: start_cursor.column,
            row_end: self.cursor.column,
          },
        }
      }
    };
    let (token, rest) = match cur {
      ':' => {
        input = chars.as_str();
        if chars.next() == Some(':') {
          (Token::DoubleColon, chars.as_str())
        } else {
          (Token::Separator(cur), input)
        }
      }
      ';' | ',' => (Token::Separator(cur), chars.as_str()),
      '.' => {
        let og_chars = chars.as_str();
        match chars.next() {
          Some('0'..='9') => self.consume_number(),
          _ => (Token::Separator(cur), og_chars),
        }
      }
      '(' | ')' | '{' | '}' => (Token::Paren(cur), chars.as_str()),
      '<' | '>' => {
        input = chars.as_str();
        let next = chars.next();
        if next == Some('=') && !self.parsing_type {
          (Token::LogicalOperation(cur), chars.as_str())
        } else if next == Some(cur) && !self.parsing_type {
          (Token::ShiftOperation(cur), chars.as_str())
        } else {
          (Token::Paren(cur), input)
        }
      }
      '[' | ']' => {
        input = chars.as_str();
        if chars.next() == Some(cur) {
          (Token::DoubleParen(cur), chars.as_str())
        } else {
          (Token::Paren(cur), input)
        }
      }
      '0'..='9' => self.consume_number(),
      'a'..='z' | 'A'..='Z' | '_' => {
        let (word, rest) = self.consume_any(|c| c.is_ascii_alphanumeric() || c == '_');
        match word {
          "true" => (Token::Bool(true), rest),
          "false" => (Token::Bool(false), rest),
          "if" => (Token::Keyword(Keyword::If), rest),
          "elseif" => (Token::Keyword(Keyword::ElseIf), rest),
          "else" => (Token::Keyword(Keyword::Else), rest),
          "while" => (Token::Keyword(Keyword::While), rest),
          "for" => (Token::Keyword(Keyword::For), rest),
          "return" => (Token::Keyword(Keyword::Return), rest),
          "break" => (Token::Keyword(Keyword::Break), rest),
          "continue" => (Token::Keyword(Keyword::Continue), rest),
          "var" => (
            Token::Keyword(Keyword::Declare(DeclarationType::Variable)),
            rest,
          ),
          "let" => (
            Token::Keyword(Keyword::Declare(DeclarationType::Const)),
            rest,
          ),
          "f32" | "u32" | "i32" | "bool" | "vec2" | "vec3" | "vec4" => {
            (Token::BuiltInType(word), rest)
          }
          "fn" => (Token::Keyword(Keyword::Function), rest),
          _ => (Token::Word(word), rest),
        }
      }
      '"' => {
        let mut iter = chars.as_str().splitn(2, '"');

        // splitn returns an iterator with at least one element, so unwrapping is fine
        let quote_content = iter.next().unwrap();
        if let Some(rest) = iter.next() {
          (Token::String(quote_content), rest)
        } else {
          (Token::UnterminatedString, quote_content)
        }
      }
      '/' if chars.as_str().starts_with('/') => {
        let _ = chars.position(|c| c == '\n' || c == '\r');
        (Token::Trivia, chars.as_str())
      }
      '-' => {
        let og_chars = chars.as_str();
        match chars.next() {
          Some('>') => (Token::Arrow, chars.as_str()),
          Some('0'..='9') | Some('.') => self.consume_number(),
          _ => (Token::Operation(cur), og_chars),
        }
      }
      '+' | '*' | '/' | '%' | '^' => (Token::Operation(cur), chars.as_str()),
      '!' => {
        input = chars.as_str();
        if chars.next() == Some('=') {
          (Token::LogicalOperation(cur), chars.as_str())
        } else {
          (Token::Operation(cur), input)
        }
      }
      '=' | '&' | '|' => {
        input = chars.as_str();
        if chars.next() == Some(cur) {
          (Token::LogicalOperation(cur), chars.as_str())
        } else {
          (Token::Operation(cur), input)
        }
      }
      ' ' | '\n' | '\r' | '\t' => {
        let (_, rest) = self.consume_any(|c| c == ' ' || c == '\n' || c == '\r' || c == '\t');
        (Token::Trivia, rest)
      }
      _ => (Token::Unknown(cur), chars.as_str()),
    };
    self.input = rest;
    TokenSpan {
      token,
      range: SourceRange {
        column_range: start_cursor.row..self.cursor.column,
        row_start: start_cursor.column,
        row_end: self.cursor.column,
      },
    }
  }

  fn consume_any(&mut self, what: impl Fn(char) -> bool) -> (&'a str, &'a str) {
    let input = self.input;
    let pos = input.find(|c| !what(c)).unwrap_or(input.len());
    input.split_at(pos)
  }

  fn consume_number(&mut self) -> (Token<'a>, &'a str) {
    let input = self.input;
    //Note: I wish this function was simpler and faster...
    let mut is_first_char = true;
    let mut right_after_exponent = false;

    let mut what = |c| {
      if is_first_char {
        is_first_char = false;
        c == '-' || ('0'..='9').contains(&c) || c == '.'
      } else if c == 'e' || c == 'E' {
        right_after_exponent = true;
        true
      } else if right_after_exponent {
        right_after_exponent = false;
        ('0'..='9').contains(&c) || c == '-'
      } else {
        ('0'..='9').contains(&c) || c == '.'
      }
    };
    let pos = input.find(|c| !what(c)).unwrap_or(input.len());
    let (value, rest) = input.split_at(pos);

    let mut rest_iter = rest.chars();
    let ty = rest_iter.next().unwrap_or(' ');
    match ty {
      'u' | 'i' | 'f' => {
        let width_end = rest_iter
          .position(|c| !('0'..='9').contains(&c))
          .unwrap_or(rest.len() - 1);
        let (width, rest) = rest[1..].split_at(width_end);
        (Token::Number { value, ty, width }, rest)
      }
      // default to `i32` or `f32`
      _ => (
        Token::Number {
          value,
          ty: if value.contains('.') { 'f' } else { 'i' },
          width: "",
        },
        rest,
      ),
    }
  }
}
