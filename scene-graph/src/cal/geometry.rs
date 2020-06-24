#[derive(Copy, Clone)]
pub struct CALAttributeTypeId(u32);

pub struct CALVertexBufferLayout {
  pub stride: i32,
  pub attributes: Vec<CALVertexAttributeBufferDescriptor>,
}

pub struct CALVertexAttributeBufferDescriptor {
  pub offset: i32,
  pub size: i32,
  pub data_type: CALVertexAttributeDataType,
}

pub enum CALVertexAttributeDataType {
  Float,
}

// impl WebGLVertexAttributeDataType {
//   pub fn to_webgl(&self) -> u32 {
//     match self {
//       Self::Float => WebGl2RenderingContext::FLOAT,
//     }
//   }
// }
