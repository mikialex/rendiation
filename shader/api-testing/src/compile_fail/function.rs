/// the returned value must be the return type of the function
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case(cx: &FunctionBuildCtx<f32>) {
///   cx.do_return(val(1_u32));
/// }
/// ```
pub struct ReturnTypeMismatch;

/// the function parameter must be sized, the runtime sized array can not be passed by value
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case(cx: &FunctionBuildCtx<f32>) {
///   cx.push_fn_parameter::<[u32]>();
/// }
/// ```
pub struct UnsizedParameter;

/// the function return type must be sized, the runtime sized array can not be returned
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case() {
///   get_shader_fn::<[u32]>(String::new());
/// }
/// ```
pub struct UnsizedReturn;

/// the function without return value is not supported
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case() {
///   get_shader_fn::<AnyType>(String::new());
/// }
/// ```
pub struct WithoutReturnValue;

/// the function without return value is not supported, the macro reports it by `compile_error!`
/// which has no error code
/// ```compile_fail
/// use rendiation_shader_api::*;
/// #[shader_fn]
/// fn case(a: Node<f32>) {}
/// ```
pub struct MacroWithoutReturnValue;

/// the parameters of the macro style function must be nodes
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// #[shader_fn]
/// fn case(a: f32) -> Node<f32> {
///   val(a)
/// }
/// ```
pub struct MacroNonNodeParameter;

/// the return type of the macro style function must be a node
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// #[shader_fn]
/// fn case(a: Node<f32>) -> f32 {
///   1.
/// }
/// ```
pub struct MacroNonNodeReturn;
