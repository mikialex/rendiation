use crate::*;

// cargo test -- --nocapture
// can print log in unit test, but have some order issue
pub fn parse(input: &str) -> Expression {
  let r = Expression::parse(&mut Lexer::new(input)).unwrap();
  println!("{:#?}", r);
  r
}

#[test]
fn parse_expression_test() {
  parse("1");
  parse("true");
  parse("!(true)");
  parse("1+1");
  parse("1+1+1");
  parse("1+(1)");
  parse("1+1*  3");
  parse("1+ -1*  - 3");
  parse("(1+ -1)*  (- 3 / 4)");
  parse("(1+ -1)*  (- test / ddd )");
  parse("(1+ -1)*  (- test(1, 2, 1/5, mu) / ddd )");
  parse("2 - 4 - 5");
  // parse("2-4-5"); fixme
  parse(" 1 < 2");
  parse(" 1 < 2 == 2");

  parse(" false && false");
  parse(" 1 & 2 == 1 || false ");

  parse("test[1]");
  parse("test2[1]/2");
  parse("test2.ui");
  parse("test3[1][3].xyz");

  parse("a= b");
  parse("a= 2");
  parse("a= b = c= 2");
  parse("a= b = c= 2 + 1 * 4");
}

fn test_parse_statement(input: &str) -> Statement {
  let r = Statement::parse(&mut Lexer::new(input)).unwrap();
  println!("{:#?}", r);
  r
}

#[test]
fn parse_st_test() {
  test_parse_statement("return 1;");
  test_parse_statement("{}");
  test_parse_statement("{;}");
  test_parse_statement("a = 2; ");
  test_parse_statement("(1+ 1); ");
  test_parse_statement("let a = (1+ 1); ");
  test_parse_statement("var a = (1+ 1); ");
  test_parse_statement(
    "
    if 1+1 {
        if false {
            return test;
        }
    } elseif test2 {
        return 9;
    }  else {
        return x;
    }

    ",
  );

  test_parse_statement(
    "
        for let i = 0; false; false   {
            print();
        }
    ",
  );
}

fn test_parse_function(input: &str) -> FunctionDefine {
  let r = FunctionDefine::parse(&mut Lexer::new(input)).unwrap();
  println!("{:#?}", r);
  r
}

#[test]
fn parse_function_test() {
  test_parse_function(
    "
  fn edge_intensity(uv: vec2<f32>) -> f32 {
    var x_step: f32 = pass_info.texel_size.x * highlighter.width;
    var y_step: f32 = pass_info.texel_size.y * highlighter.width;

    var all: f32 = 0.0;
    all = all + textureSample(mask, tex_sampler, in.uv).x;
    all = all + textureSample(mask, tex_sampler, vec2<f32>(in.uv.x + x_step, in.uv.y)).x;
    all = all + textureSample(mask, tex_sampler, vec2<f32>(in.uv.x, in.uv.y + y_step)).x;
    all = all + textureSample(mask, tex_sampler, vec2<f32>(in.uv.x + x_step, in.uv.y+ y_step)).x;

    var intensity = (1.0 - 2.0 * abs(all / 4. - 0.5)) * highlighter.color.a;
  }
  ",
  );
}
