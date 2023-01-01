use crate::*;

// cargo test -- --nocapture
// can print log in unit test, but have some order issue
pub fn parse(input: &str) -> Expression {
  let r = Expression::parse_input(input).unwrap();
  println!("{r:#?}");
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
  let r = Statement::parse_input(input).unwrap();
  println!("{r:#?}");
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
    } else if test2 {
        return 9;
    }  else {
        return x;
    }

    ",
  );

  test_parse_statement(
    "
        for (let i = 0; false; i++)   {
            print();
        }
    ",
  );
}

fn test_parse_function(input: &str) -> FunctionDefine {
  let r = FunctionDefine::parse_input(input).unwrap();
  println!("{r:#?}");
  r
}

#[test]
fn parse_function_test1() {
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

#[test]
fn parse_function_test2() {
  test_parse_function(
    "
  fn background_direction(vertex_index: u32, view: mat4x4<f32>, projection_inv: mat4x4<f32>) -> vec3<f32> {
    // hacky way to draw a large triangle
    let tmp1 = i32(vertex_index) / 2;
    let tmp2 = i32(vertex_index) & 1;
    let pos = vec4<f32>(
        f32(tmp1) * 4.0 - 1.0,
        f32(tmp2) * 4.0 - 1.0,
        1.0,
        1.0
    );

    // transposition = inversion for this orthonormal matrix
    let inv_model_view = transpose(mat3x3<f32>(view.x.xyz, view.y.xyz, view.z.xyz));
    let unprojected = projection_inv * pos;

    return inv_model_view * unprojected.xyz;
  }
  ",
  );
}

#[test]
fn parse_function_test3() {
  test_parse_function(
    "
  fn fatline_round_corner(uv: vec2<f32>) {
    if (abs(vUv.y) > 1.0) {
      let a = vUv.x;
      let b: f32;
      if (vUv.y > 0.0) {
        b = vUv.y - 1.0;
      } else {
        b = vUv.y + 1.0;
      }
      let len2 = a * a + b * b;
      if (len2 > 1.0) {
        discard;
      }
    }
  }
  ",
  );
}

#[test]
fn parse_function_test4() {
  test_parse_function(
    "
    fn generate_quad(
      vertex_index: u32
    ) -> VertexOut {
      var left: f32 = -1.0;
      var right: f32 = 1.0;
      var top: f32 = 1.0;
      var bottom: f32 = -1.0;
      var depth: f32 = 0.0;
  
      switch (i32(vertex_index)) {
        case 0: {
          out.position = vec4<f32>(left, top, depth, 1.);
          out.uv = vec2<f32>(0., 0.);
        }
        case 1: {
          out.position = vec4<f32>(right, top, depth, 1.);
          out.uv = vec2<f32>(1., 0.);
        }
        case 2: {
          out.position = vec4<f32>(left, bottom, depth, 1.);
          out.uv = vec2<f32>(0., 1.);
        }
        default: {
          out.position = vec4<f32>(right, bottom, depth, 1.);
          out.uv = vec2<f32>(1., 1.);
        }
      }
    }
  ",
  );
}

#[test]
fn parse_function_test5() {
  test_parse_function(
    "
    fn linear_blur(
      direction: vec2<f32>,
      weights: ShaderSamplingWeights,
      texture: texture_2d<f32>,
      sp: sampler,
      uv: vec2<f32>,
      texel_size: vec2<f32>
    ) -> f32 {
      let sample_offset = texel_size * direction;
      var sum: vec4<f32>;
      for (var i: i32 = 2; i < weights.weight_count; i++) {
          let samples = textureSample(texture, sp, uv + f32(i) * sample_offset);
          sum = lin_space(1.0, sum, weights.weights[i], samples);
      }
    }
  ",
  );
}
