/// the filtered item is stored in a variable, the pointer item can not be stored, map it to the
/// value first
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// fn case(array: ShaderPtrOf<[u32; 4]>) {
///   array
///     .into_shader_iter()
///     .filter(|_| val(true))
///     .for_each(|_, _| {});
/// }
/// ```
pub struct FilterPointerItem;

/// the zipped value must be a shader iterator, call into_shader_iter first
/// ```compile_fail,E0277
/// use rendiation_shader_api::*;
/// fn case(n: Node<u32>) {
///   n.into_shader_iter().zip(n);
/// }
/// ```
pub struct ZipIntoShaderIterator;
