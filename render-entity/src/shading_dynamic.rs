pub struct DynamicShading {
    pub index: usize,
    pub vertex_str: String,
    pub frag_str: String,
    pub attributes: Vec<String>,
    pub uniforms: Vec<String>,
  }
  
  impl DynamicShading {
    pub fn new(
      index: usize,
      vertex_str: String,
      frag_str: String,
      attributes: Vec<String>,
      uniforms: Vec<String>,
    ) -> DynamicShading {
      DynamicShading {
        index,
        vertex_str,
        frag_str,
        attributes,
        uniforms,
      }
    }
  }
  