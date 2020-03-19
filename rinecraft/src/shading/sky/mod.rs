use rendiation_math::*;

pub fn shader_function(func: &str) {}

pub trait ShaderFunction<I, O> {
  const shader:  &'static str;
  // fn get_shader() -> &'static str;
}

pub trait ShaderDataType {}

impl ShaderDataType for f32 {}
impl ShaderDataType for Vec2<f32> {}
impl ShaderDataType for Vec3<f32> {}

// pub struct ShaderFunctionInput(Vec2<f32>, Vec3<f32>);
// pub struct ShaderFunctionInput(Vec2<f32>, Vec3<f32>);

pub trait ShaderFunctionInput {}

pub struct ShaderFunctionInput1<T: ShaderDataType>(T);
// impl ShaderFunctionInput for ShaderFunctionInput1{}

pub struct ShaderFunctionInput2<T1: ShaderDataType, T2: ShaderDataType>(T1, T2);



struct HgPhaseShaderFunction;
impl ShaderFunction<ShaderFunctionInput2<f32, f32>, f32> for HgPhaseShaderFunction {
  const shader: &'static str = r#"
  float hgPhase( float cosTheta, float g ) {
      float g2 = pow( g, 2.0 );
    float inverse = 1.0 / pow( 1.0 - 2.0 * g * cosTheta + g2, 1.5 );
    
    // 1.0 / ( 4.0 * pi )
    const float ONE_OVER_FOURPI = 0.07957747154594767;
      return ONE_OVER_FOURPI * ( ( 1.0 - g2 ) * inverse );
  }
  "#;
}

struct ShaderComputeNode<T>{
    factory: T,
}

impl ShaderComputeNode<HgPhaseShaderFunction>{
    pub fn input_cosTheta<I, U: ShaderFunction<I, f32>>(node: ShaderComputeNode<U>){
        
    }
}

pub trait ShaderNode{
  
}