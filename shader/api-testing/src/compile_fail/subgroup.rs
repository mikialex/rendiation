/// subgroup arithmetic operations require numeric type
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// let v = val(true).subgroup_add();
/// ```
pub struct SubgroupAddBool;

/// subgroup communication operations require numeric type
/// ```compile_fail,E0599
/// use rendiation_shader_api::*;
/// let v = val(true).subgroup_shuffle(val(0));
/// ```
pub struct SubgroupShuffleBool;
