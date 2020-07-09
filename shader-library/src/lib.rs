use rendiation_shadergraph_derives::glsl_function;

glsl_function!(
"
vec3 uncharted2ToneMapping(
  vec3 intensity, 
  float toneMappingExposure,
  float toneMappingWhitePoint
) {
  intensity *= toneMappingExposure;
  return Uncharted2Helper(intensity) / Uncharted2Helper(vec3(toneMappingWhitePoint));
}

");

#[test]
fn test(){
  let a = uncharted2ToneMappingFunction::name();
  println!("{}", uncharted2ToneMappingFunction::name());
}