use std::collections::HashMap;

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, Linkage, Module};
use rendiation_shader_api::ShaderAPI;

pub struct ShaderAPICraneliftBackend {
  /// The function builder context, which is reused across multiple
  /// FunctionBuilder instances.
  builder_context: FunctionBuilderContext,

  /// The main Cranelift context, which holds the state for codegen. Cranelift
  /// separates this from `Module` to allow for parallel compilation, with a
  /// context per thread, though this isn't in the simple demo here.
  ctx: codegen::Context,

  /// The data description, which is to data objects what `ctx` is to functions.
  data_description: DataDescription,

  /// The module, with the jit backend, which manages the JIT'd
  /// functions.
  module: JITModule,
}

impl Default for ShaderAPICraneliftBackend {
  fn default() -> Self {
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    flag_builder.set("is_pic", "false").unwrap();

    let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
      panic!("host machine is not supported: {}", msg);
    });
    let isa = isa_builder
      .finish(settings::Flags::new(flag_builder))
      .unwrap();
    let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

    let module = JITModule::new(builder);
    Self {
      builder_context: FunctionBuilderContext::new(),
      ctx: module.make_context(),
      data_description: DataDescription::new(),
      module,
    }
  }
}

impl ShaderAPICraneliftBackend {
  pub fn compile(&mut self) -> Result<*const u8, String> {
    let int = Type::int(64).unwrap();

    self.ctx.func.signature.params.push(AbiParam::new(int));
    self.ctx.func.signature.returns.push(AbiParam::new(int));

    // Create the builder to build a function.
    let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);

    // Create the entry block, to start emitting code in.
    let entry_block = builder.create_block();

    // Since this is the entry block, add block parameters corresponding to
    // the function's parameters.
    //
    // TODO: Streamline the API here.
    builder.append_block_params_for_function_params(entry_block);

    // Tell the builder to emit code in this block.
    builder.switch_to_block(entry_block);

    // And, tell the builder that this block will have no further
    // predecessors. Since it's the entry block, it won't have any
    // predecessors.
    builder.seal_block(entry_block);

    ///////////

    let param0_value = builder.block_params(entry_block)[0];
    // let var = declare_variable(int, builder, &mut variables, &mut index, name);
    // builder.def_var(var, param0_value);

    let a = builder.ins().iconst(int, i64::from(1));

    let r = builder.ins().iadd(param0_value, a);
    builder.ins().return_(&[r]);

    ///////////

    // Next, declare the function to jit. Functions must be declared
    // before they can be called, or defined.
    //
    // TODO: This may be an area where the API should be streamlined; should
    // we have a version of `declare_function` that automatically declares
    // the function?
    let id = self
      .module
      .declare_function("test", Linkage::Export, &self.ctx.func.signature)
      .map_err(|e| e.to_string())?;

    // Define the function to jit. This finishes compilation, although
    // there may be outstanding relocations to perform. Currently, jit
    // cannot finish relocations until all functions to be called are
    // defined. For this toy demo for now, we'll just finalize the
    // function below.
    self
      .module
      .define_function(id, &mut self.ctx)
      .map_err(|e| e.to_string())?;

    // Finalize the functions which we just defined, which resolves any
    // outstanding relocations (patching in addresses, now that they're
    // available).
    self.module.finalize_definitions().unwrap();

    // We can now retrieve a pointer to the machine code.
    let code = self.module.get_finalized_function(id);

    Ok(code)
  }

  pub fn test(&mut self) {
    let code_ptr = self.compile().unwrap();
    let function = unsafe { std::mem::transmute::<_, fn(i64) -> i64>(code_ptr) };

    println!("{}", function(1));
    println!("{}", function(3));
  }
}

#[test]
fn test() {
  let mut backend = ShaderAPICraneliftBackend::default();
  backend.test();
}
