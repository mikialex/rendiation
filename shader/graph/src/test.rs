use crate::*;

use crate as shadergraph;

glsl_function!(
  "
    vec3 importanceSampleCone(const in vec2 coords, const in float cosConeAngle) {
      float phi = TWO_PI * coords.x;
    
      float cosTheta = cosConeAngle * (1.0 - coords.y) + coords.y;
      float sinTheta = sqrt(max(0.0001, 1.0 - cosTheta * cosTheta));
    
      vec3 h;
      h.x = sinTheta * cos(phi);
      h.y = sinTheta * sin(phi);
      h.z = cosTheta;
    
      return h;
    }
  "
);

glsl_function!(
  "
  float linstep(float minVal, float maxVal, float val) {
    // NOTE: smoothstep is important to the seams introduced by the cubemap
    return smoothstep(minVal, maxVal, val);
  }
  "
);

glsl_function!(
  "
  float reduceLightBleeding(float p_max, float amount) {
    return linstep(amount, 1.0, p_max);
    }
  "
);

glsl_function!(
    "
  float chebyshevUpperBound(vec2 moments, float mean, float minVariance, float lightBleedingReduction)
  {
  // Compute variance
  float variance = moments.y - (moments.x * moments.x);
  variance = max(variance, minVariance);
  
  // Compute probabilistic upper bound
  float d = mean - moments.x;
  float pMax = variance / (variance + (d * d));
  
  pMax = reduceLightBleeding(pMax, lightBleedingReduction);
  
  // One-tailed Chebyshev
  return (mean <= moments.x ? 1.0 : pMax);
  }
  "
  );

// impl ShaderIterator for Node<ShaderArray<T>> {
//   type Item = T;

//   fn code_gen(&self) -> &'static str {
//     "
//         for(int i = 0; i < 32; i++) {

//         }
//         "
//   }
// }

// let a = 1;
// let c = 0;
// for i in xxx {
//     let b =1;
//     if i> 10 {
//         a+=b
//         continue
//     }
//     c+= i;
// }

// fn test() {
//   let a = node(1);
//   let c = node(0);
//   let b = node(1);
//   xxx.iter().split(
//     until(10).fold(a, |a| a + b),
//     enumerate().fold(a, |a, i| a + i),
//   );
// }

fn test() {
  let a = consts(1).mutable();
  let c = consts(0).mutable();

  for_by(5, |for_ctx, i| {
    let b = 1;
    if_by(i.greater_than(0), || {
      a.set(a.get() + b.into());
      for_ctx.do_continue();
    });
    c.set(c.get() + i);
  });
}
