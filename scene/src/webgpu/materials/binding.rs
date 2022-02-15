use std::{
  any::Any,
  cell::RefCell,
  collections::{hash_map::DefaultHasher, HashMap},
  hash::{Hash, Hasher},
  rc::Rc,
};

use rendiation_webgpu::{GPURenderPass, GPURenderPipeline};
use shadergraph::SemanticShaderUniform;
