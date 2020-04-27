// todo write marco do this;

use crate::{Vec4, Vec2, Vec3};

impl<T> Vec4<T> where T: Copy
{
#[inline(always)]
pub fn xx(&self) -> Vec2<T> { Vec2::new(self.x, self.x) }
#[inline(always)]
pub fn xy(&self) -> Vec2<T> { Vec2::new(self.x, self.y) }
#[inline(always)]
pub fn xz(&self) -> Vec2<T> { Vec2::new(self.x, self.z) }
#[inline(always)]
pub fn xw(&self) -> Vec2<T> { Vec2::new(self.x, self.w) }
#[inline(always)]
pub fn yx(&self) -> Vec2<T> { Vec2::new(self.y, self.x) }
#[inline(always)]
pub fn yy(&self) -> Vec2<T> { Vec2::new(self.y, self.y) }
#[inline(always)]
pub fn yz(&self) -> Vec2<T> { Vec2::new(self.y, self.z) }
#[inline(always)]
pub fn yw(&self) -> Vec2<T> { Vec2::new(self.y, self.w) }
#[inline(always)]
pub fn zx(&self) -> Vec2<T> { Vec2::new(self.z, self.x) }
#[inline(always)]
pub fn zy(&self) -> Vec2<T> { Vec2::new(self.z, self.y) }
#[inline(always)]
pub fn zz(&self) -> Vec2<T> { Vec2::new(self.z, self.z) }
#[inline(always)]
pub fn zw(&self) -> Vec2<T> { Vec2::new(self.z, self.w) }
#[inline(always)]
pub fn wx(&self) -> Vec2<T> { Vec2::new(self.w, self.x) }
#[inline(always)]
pub fn wy(&self) -> Vec2<T> { Vec2::new(self.w, self.y) }
#[inline(always)]
pub fn wz(&self) -> Vec2<T> { Vec2::new(self.w, self.z) }
#[inline(always)]
pub fn ww(&self) -> Vec2<T> { Vec2::new(self.w, self.w) }
#[inline(always)]
pub fn xxx(&self) -> Vec3<T> { Vec3::new(self.x, self.x, self.x) }
#[inline(always)]
pub fn xxy(&self) -> Vec3<T> { Vec3::new(self.x, self.x, self.y) }
#[inline(always)]
pub fn xxz(&self) -> Vec3<T> { Vec3::new(self.x, self.x, self.z) }
#[inline(always)]
pub fn xxw(&self) -> Vec3<T> { Vec3::new(self.x, self.x, self.w) }
#[inline(always)]
pub fn xyx(&self) -> Vec3<T> { Vec3::new(self.x, self.y, self.x) }
#[inline(always)]
pub fn xyy(&self) -> Vec3<T> { Vec3::new(self.x, self.y, self.y) }
#[inline(always)]
pub fn xyz(&self) -> Vec3<T> { Vec3::new(self.x, self.y, self.z) }
#[inline(always)]
pub fn xyw(&self) -> Vec3<T> { Vec3::new(self.x, self.y, self.w) }
#[inline(always)]
pub fn xzx(&self) -> Vec3<T> { Vec3::new(self.x, self.z, self.x) }
#[inline(always)]
pub fn xzy(&self) -> Vec3<T> { Vec3::new(self.x, self.z, self.y) }
#[inline(always)]
pub fn xzz(&self) -> Vec3<T> { Vec3::new(self.x, self.z, self.z) }
#[inline(always)]
pub fn xzw(&self) -> Vec3<T> { Vec3::new(self.x, self.z, self.w) }
#[inline(always)]
pub fn xwx(&self) -> Vec3<T> { Vec3::new(self.x, self.w, self.x) }
#[inline(always)]
pub fn xwy(&self) -> Vec3<T> { Vec3::new(self.x, self.w, self.y) }
#[inline(always)]
pub fn xwz(&self) -> Vec3<T> { Vec3::new(self.x, self.w, self.z) }
#[inline(always)]
pub fn xww(&self) -> Vec3<T> { Vec3::new(self.x, self.w, self.w) }
#[inline(always)]
pub fn yxx(&self) -> Vec3<T> { Vec3::new(self.y, self.x, self.x) }
#[inline(always)]
pub fn yxy(&self) -> Vec3<T> { Vec3::new(self.y, self.x, self.y) }
#[inline(always)]
pub fn yxz(&self) -> Vec3<T> { Vec3::new(self.y, self.x, self.z) }
#[inline(always)]
pub fn yxw(&self) -> Vec3<T> { Vec3::new(self.y, self.x, self.w) }
#[inline(always)]
pub fn yyx(&self) -> Vec3<T> { Vec3::new(self.y, self.y, self.x) }
#[inline(always)]
pub fn yyy(&self) -> Vec3<T> { Vec3::new(self.y, self.y, self.y) }
#[inline(always)]
pub fn yyz(&self) -> Vec3<T> { Vec3::new(self.y, self.y, self.z) }
#[inline(always)]
pub fn yyw(&self) -> Vec3<T> { Vec3::new(self.y, self.y, self.w) }
#[inline(always)]
pub fn yzx(&self) -> Vec3<T> { Vec3::new(self.y, self.z, self.x) }
#[inline(always)]
pub fn yzy(&self) -> Vec3<T> { Vec3::new(self.y, self.z, self.y) }
#[inline(always)]
pub fn yzz(&self) -> Vec3<T> { Vec3::new(self.y, self.z, self.z) }
#[inline(always)]
pub fn yzw(&self) -> Vec3<T> { Vec3::new(self.y, self.z, self.w) }
#[inline(always)]
pub fn ywx(&self) -> Vec3<T> { Vec3::new(self.y, self.w, self.x) }
#[inline(always)]
pub fn ywy(&self) -> Vec3<T> { Vec3::new(self.y, self.w, self.y) }
#[inline(always)]
pub fn ywz(&self) -> Vec3<T> { Vec3::new(self.y, self.w, self.z) }
#[inline(always)]
pub fn yww(&self) -> Vec3<T> { Vec3::new(self.y, self.w, self.w) }
#[inline(always)]
pub fn zxx(&self) -> Vec3<T> { Vec3::new(self.z, self.x, self.x) }
#[inline(always)]
pub fn zxy(&self) -> Vec3<T> { Vec3::new(self.z, self.x, self.y) }
#[inline(always)]
pub fn zxz(&self) -> Vec3<T> { Vec3::new(self.z, self.x, self.z) }
#[inline(always)]
pub fn zxw(&self) -> Vec3<T> { Vec3::new(self.z, self.x, self.w) }
#[inline(always)]
pub fn zyx(&self) -> Vec3<T> { Vec3::new(self.z, self.y, self.x) }
#[inline(always)]
pub fn zyy(&self) -> Vec3<T> { Vec3::new(self.z, self.y, self.y) }
#[inline(always)]
pub fn zyz(&self) -> Vec3<T> { Vec3::new(self.z, self.y, self.z) }
#[inline(always)]
pub fn zyw(&self) -> Vec3<T> { Vec3::new(self.z, self.y, self.w) }
#[inline(always)]
pub fn zzx(&self) -> Vec3<T> { Vec3::new(self.z, self.z, self.x) }
#[inline(always)]
pub fn zzy(&self) -> Vec3<T> { Vec3::new(self.z, self.z, self.y) }
#[inline(always)]
pub fn zzz(&self) -> Vec3<T> { Vec3::new(self.z, self.z, self.z) }
#[inline(always)]
pub fn zzw(&self) -> Vec3<T> { Vec3::new(self.z, self.z, self.w) }
#[inline(always)]
pub fn zwx(&self) -> Vec3<T> { Vec3::new(self.z, self.w, self.x) }
#[inline(always)]
pub fn zwy(&self) -> Vec3<T> { Vec3::new(self.z, self.w, self.y) }
#[inline(always)]
pub fn zwz(&self) -> Vec3<T> { Vec3::new(self.z, self.w, self.z) }
#[inline(always)]
pub fn zww(&self) -> Vec3<T> { Vec3::new(self.z, self.w, self.w) }
#[inline(always)]
pub fn wxx(&self) -> Vec3<T> { Vec3::new(self.w, self.x, self.x) }
#[inline(always)]
pub fn wxy(&self) -> Vec3<T> { Vec3::new(self.w, self.x, self.y) }
#[inline(always)]
pub fn wxz(&self) -> Vec3<T> { Vec3::new(self.w, self.x, self.z) }
#[inline(always)]
pub fn wxw(&self) -> Vec3<T> { Vec3::new(self.w, self.x, self.w) }
#[inline(always)]
pub fn wyx(&self) -> Vec3<T> { Vec3::new(self.w, self.y, self.x) }
#[inline(always)]
pub fn wyy(&self) -> Vec3<T> { Vec3::new(self.w, self.y, self.y) }
#[inline(always)]
pub fn wyz(&self) -> Vec3<T> { Vec3::new(self.w, self.y, self.z) }
#[inline(always)]
pub fn wyw(&self) -> Vec3<T> { Vec3::new(self.w, self.y, self.w) }
#[inline(always)]
pub fn wzx(&self) -> Vec3<T> { Vec3::new(self.w, self.z, self.x) }
#[inline(always)]
pub fn wzy(&self) -> Vec3<T> { Vec3::new(self.w, self.z, self.y) }
#[inline(always)]
pub fn wzz(&self) -> Vec3<T> { Vec3::new(self.w, self.z, self.z) }
#[inline(always)]
pub fn wzw(&self) -> Vec3<T> { Vec3::new(self.w, self.z, self.w) }
#[inline(always)]
pub fn wwx(&self) -> Vec3<T> { Vec3::new(self.w, self.w, self.x) }
#[inline(always)]
pub fn wwy(&self) -> Vec3<T> { Vec3::new(self.w, self.w, self.y) }
#[inline(always)]
pub fn wwz(&self) -> Vec3<T> { Vec3::new(self.w, self.w, self.z) }
#[inline(always)]
pub fn www(&self) -> Vec3<T> { Vec3::new(self.w, self.w, self.w) }
#[inline(always)]
pub fn xxxx(&self) -> Self { Self::new(self.x, self.x, self.x, self.x) }
#[inline(always)]
pub fn xxxy(&self) -> Self { Self::new(self.x, self.x, self.x, self.y) }
#[inline(always)]
pub fn xxxz(&self) -> Self { Self::new(self.x, self.x, self.x, self.z) }
#[inline(always)]
pub fn xxxw(&self) -> Self { Self::new(self.x, self.x, self.x, self.w) }
#[inline(always)]
pub fn xxyx(&self) -> Self { Self::new(self.x, self.x, self.y, self.x) }
#[inline(always)]
pub fn xxyy(&self) -> Self { Self::new(self.x, self.x, self.y, self.y) }
#[inline(always)]
pub fn xxyz(&self) -> Self { Self::new(self.x, self.x, self.y, self.z) }
#[inline(always)]
pub fn xxyw(&self) -> Self { Self::new(self.x, self.x, self.y, self.w) }
#[inline(always)]
pub fn xxzx(&self) -> Self { Self::new(self.x, self.x, self.z, self.x) }
#[inline(always)]
pub fn xxzy(&self) -> Self { Self::new(self.x, self.x, self.z, self.y) }
#[inline(always)]
pub fn xxzz(&self) -> Self { Self::new(self.x, self.x, self.z, self.z) }
#[inline(always)]
pub fn xxzw(&self) -> Self { Self::new(self.x, self.x, self.z, self.w) }
#[inline(always)]
pub fn xxwx(&self) -> Self { Self::new(self.x, self.x, self.w, self.x) }
#[inline(always)]
pub fn xxwy(&self) -> Self { Self::new(self.x, self.x, self.w, self.y) }
#[inline(always)]
pub fn xxwz(&self) -> Self { Self::new(self.x, self.x, self.w, self.z) }
#[inline(always)]
pub fn xxww(&self) -> Self { Self::new(self.x, self.x, self.w, self.w) }
#[inline(always)]
pub fn xyxx(&self) -> Self { Self::new(self.x, self.y, self.x, self.x) }
#[inline(always)]
pub fn xyxy(&self) -> Self { Self::new(self.x, self.y, self.x, self.y) }
#[inline(always)]
pub fn xyxz(&self) -> Self { Self::new(self.x, self.y, self.x, self.z) }
#[inline(always)]
pub fn xyxw(&self) -> Self { Self::new(self.x, self.y, self.x, self.w) }
#[inline(always)]
pub fn xyyx(&self) -> Self { Self::new(self.x, self.y, self.y, self.x) }
#[inline(always)]
pub fn xyyy(&self) -> Self { Self::new(self.x, self.y, self.y, self.y) }
#[inline(always)]
pub fn xyyz(&self) -> Self { Self::new(self.x, self.y, self.y, self.z) }
#[inline(always)]
pub fn xyyw(&self) -> Self { Self::new(self.x, self.y, self.y, self.w) }
#[inline(always)]
pub fn xyzx(&self) -> Self { Self::new(self.x, self.y, self.z, self.x) }
#[inline(always)]
pub fn xyzy(&self) -> Self { Self::new(self.x, self.y, self.z, self.y) }
#[inline(always)]
pub fn xyzz(&self) -> Self { Self::new(self.x, self.y, self.z, self.z) }
#[inline(always)]
pub fn xyzw(&self) -> Self { Self::new(self.x, self.y, self.z, self.w) }
#[inline(always)]
pub fn xywx(&self) -> Self { Self::new(self.x, self.y, self.w, self.x) }
#[inline(always)]
pub fn xywy(&self) -> Self { Self::new(self.x, self.y, self.w, self.y) }
#[inline(always)]
pub fn xywz(&self) -> Self { Self::new(self.x, self.y, self.w, self.z) }
#[inline(always)]
pub fn xyww(&self) -> Self { Self::new(self.x, self.y, self.w, self.w) }
#[inline(always)]
pub fn xzxx(&self) -> Self { Self::new(self.x, self.z, self.x, self.x) }
#[inline(always)]
pub fn xzxy(&self) -> Self { Self::new(self.x, self.z, self.x, self.y) }
#[inline(always)]
pub fn xzxz(&self) -> Self { Self::new(self.x, self.z, self.x, self.z) }
#[inline(always)]
pub fn xzxw(&self) -> Self { Self::new(self.x, self.z, self.x, self.w) }
#[inline(always)]
pub fn xzyx(&self) -> Self { Self::new(self.x, self.z, self.y, self.x) }
#[inline(always)]
pub fn xzyy(&self) -> Self { Self::new(self.x, self.z, self.y, self.y) }
#[inline(always)]
pub fn xzyz(&self) -> Self { Self::new(self.x, self.z, self.y, self.z) }
#[inline(always)]
pub fn xzyw(&self) -> Self { Self::new(self.x, self.z, self.y, self.w) }
#[inline(always)]
pub fn xzzx(&self) -> Self { Self::new(self.x, self.z, self.z, self.x) }
#[inline(always)]
pub fn xzzy(&self) -> Self { Self::new(self.x, self.z, self.z, self.y) }
#[inline(always)]
pub fn xzzz(&self) -> Self { Self::new(self.x, self.z, self.z, self.z) }
#[inline(always)]
pub fn xzzw(&self) -> Self { Self::new(self.x, self.z, self.z, self.w) }
#[inline(always)]
pub fn xzwx(&self) -> Self { Self::new(self.x, self.z, self.w, self.x) }
#[inline(always)]
pub fn xzwy(&self) -> Self { Self::new(self.x, self.z, self.w, self.y) }
#[inline(always)]
pub fn xzwz(&self) -> Self { Self::new(self.x, self.z, self.w, self.z) }
#[inline(always)]
pub fn xzww(&self) -> Self { Self::new(self.x, self.z, self.w, self.w) }
#[inline(always)]
pub fn xwxx(&self) -> Self { Self::new(self.x, self.w, self.x, self.x) }
#[inline(always)]
pub fn xwxy(&self) -> Self { Self::new(self.x, self.w, self.x, self.y) }
#[inline(always)]
pub fn xwxz(&self) -> Self { Self::new(self.x, self.w, self.x, self.z) }
#[inline(always)]
pub fn xwxw(&self) -> Self { Self::new(self.x, self.w, self.x, self.w) }
#[inline(always)]
pub fn xwyx(&self) -> Self { Self::new(self.x, self.w, self.y, self.x) }
#[inline(always)]
pub fn xwyy(&self) -> Self { Self::new(self.x, self.w, self.y, self.y) }
#[inline(always)]
pub fn xwyz(&self) -> Self { Self::new(self.x, self.w, self.y, self.z) }
#[inline(always)]
pub fn xwyw(&self) -> Self { Self::new(self.x, self.w, self.y, self.w) }
#[inline(always)]
pub fn xwzx(&self) -> Self { Self::new(self.x, self.w, self.z, self.x) }
#[inline(always)]
pub fn xwzy(&self) -> Self { Self::new(self.x, self.w, self.z, self.y) }
#[inline(always)]
pub fn xwzz(&self) -> Self { Self::new(self.x, self.w, self.z, self.z) }
#[inline(always)]
pub fn xwzw(&self) -> Self { Self::new(self.x, self.w, self.z, self.w) }
#[inline(always)]
pub fn xwwx(&self) -> Self { Self::new(self.x, self.w, self.w, self.x) }
#[inline(always)]
pub fn xwwy(&self) -> Self { Self::new(self.x, self.w, self.w, self.y) }
#[inline(always)]
pub fn xwwz(&self) -> Self { Self::new(self.x, self.w, self.w, self.z) }
#[inline(always)]
pub fn xwww(&self) -> Self { Self::new(self.x, self.w, self.w, self.w) }
#[inline(always)]
pub fn yxxx(&self) -> Self { Self::new(self.y, self.x, self.x, self.x) }
#[inline(always)]
pub fn yxxy(&self) -> Self { Self::new(self.y, self.x, self.x, self.y) }
#[inline(always)]
pub fn yxxz(&self) -> Self { Self::new(self.y, self.x, self.x, self.z) }
#[inline(always)]
pub fn yxxw(&self) -> Self { Self::new(self.y, self.x, self.x, self.w) }
#[inline(always)]
pub fn yxyx(&self) -> Self { Self::new(self.y, self.x, self.y, self.x) }
#[inline(always)]
pub fn yxyy(&self) -> Self { Self::new(self.y, self.x, self.y, self.y) }
#[inline(always)]
pub fn yxyz(&self) -> Self { Self::new(self.y, self.x, self.y, self.z) }
#[inline(always)]
pub fn yxyw(&self) -> Self { Self::new(self.y, self.x, self.y, self.w) }
#[inline(always)]
pub fn yxzx(&self) -> Self { Self::new(self.y, self.x, self.z, self.x) }
#[inline(always)]
pub fn yxzy(&self) -> Self { Self::new(self.y, self.x, self.z, self.y) }
#[inline(always)]
pub fn yxzz(&self) -> Self { Self::new(self.y, self.x, self.z, self.z) }
#[inline(always)]
pub fn yxzw(&self) -> Self { Self::new(self.y, self.x, self.z, self.w) }
#[inline(always)]
pub fn yxwx(&self) -> Self { Self::new(self.y, self.x, self.w, self.x) }
#[inline(always)]
pub fn yxwy(&self) -> Self { Self::new(self.y, self.x, self.w, self.y) }
#[inline(always)]
pub fn yxwz(&self) -> Self { Self::new(self.y, self.x, self.w, self.z) }
#[inline(always)]
pub fn yxww(&self) -> Self { Self::new(self.y, self.x, self.w, self.w) }
#[inline(always)]
pub fn yyxx(&self) -> Self { Self::new(self.y, self.y, self.x, self.x) }
#[inline(always)]
pub fn yyxy(&self) -> Self { Self::new(self.y, self.y, self.x, self.y) }
#[inline(always)]
pub fn yyxz(&self) -> Self { Self::new(self.y, self.y, self.x, self.z) }
#[inline(always)]
pub fn yyxw(&self) -> Self { Self::new(self.y, self.y, self.x, self.w) }
#[inline(always)]
pub fn yyyx(&self) -> Self { Self::new(self.y, self.y, self.y, self.x) }
#[inline(always)]
pub fn yyyy(&self) -> Self { Self::new(self.y, self.y, self.y, self.y) }
#[inline(always)]
pub fn yyyz(&self) -> Self { Self::new(self.y, self.y, self.y, self.z) }
#[inline(always)]
pub fn yyyw(&self) -> Self { Self::new(self.y, self.y, self.y, self.w) }
#[inline(always)]
pub fn yyzx(&self) -> Self { Self::new(self.y, self.y, self.z, self.x) }
#[inline(always)]
pub fn yyzy(&self) -> Self { Self::new(self.y, self.y, self.z, self.y) }
#[inline(always)]
pub fn yyzz(&self) -> Self { Self::new(self.y, self.y, self.z, self.z) }
#[inline(always)]
pub fn yyzw(&self) -> Self { Self::new(self.y, self.y, self.z, self.w) }
#[inline(always)]
pub fn yywx(&self) -> Self { Self::new(self.y, self.y, self.w, self.x) }
#[inline(always)]
pub fn yywy(&self) -> Self { Self::new(self.y, self.y, self.w, self.y) }
#[inline(always)]
pub fn yywz(&self) -> Self { Self::new(self.y, self.y, self.w, self.z) }
#[inline(always)]
pub fn yyww(&self) -> Self { Self::new(self.y, self.y, self.w, self.w) }
#[inline(always)]
pub fn yzxx(&self) -> Self { Self::new(self.y, self.z, self.x, self.x) }
#[inline(always)]
pub fn yzxy(&self) -> Self { Self::new(self.y, self.z, self.x, self.y) }
#[inline(always)]
pub fn yzxz(&self) -> Self { Self::new(self.y, self.z, self.x, self.z) }
#[inline(always)]
pub fn yzxw(&self) -> Self { Self::new(self.y, self.z, self.x, self.w) }
#[inline(always)]
pub fn yzyx(&self) -> Self { Self::new(self.y, self.z, self.y, self.x) }
#[inline(always)]
pub fn yzyy(&self) -> Self { Self::new(self.y, self.z, self.y, self.y) }
#[inline(always)]
pub fn yzyz(&self) -> Self { Self::new(self.y, self.z, self.y, self.z) }
#[inline(always)]
pub fn yzyw(&self) -> Self { Self::new(self.y, self.z, self.y, self.w) }
#[inline(always)]
pub fn yzzx(&self) -> Self { Self::new(self.y, self.z, self.z, self.x) }
#[inline(always)]
pub fn yzzy(&self) -> Self { Self::new(self.y, self.z, self.z, self.y) }
#[inline(always)]
pub fn yzzz(&self) -> Self { Self::new(self.y, self.z, self.z, self.z) }
#[inline(always)]
pub fn yzzw(&self) -> Self { Self::new(self.y, self.z, self.z, self.w) }
#[inline(always)]
pub fn yzwx(&self) -> Self { Self::new(self.y, self.z, self.w, self.x) }
#[inline(always)]
pub fn yzwy(&self) -> Self { Self::new(self.y, self.z, self.w, self.y) }
#[inline(always)]
pub fn yzwz(&self) -> Self { Self::new(self.y, self.z, self.w, self.z) }
#[inline(always)]
pub fn yzww(&self) -> Self { Self::new(self.y, self.z, self.w, self.w) }
#[inline(always)]
pub fn ywxx(&self) -> Self { Self::new(self.y, self.w, self.x, self.x) }
#[inline(always)]
pub fn ywxy(&self) -> Self { Self::new(self.y, self.w, self.x, self.y) }
#[inline(always)]
pub fn ywxz(&self) -> Self { Self::new(self.y, self.w, self.x, self.z) }
#[inline(always)]
pub fn ywxw(&self) -> Self { Self::new(self.y, self.w, self.x, self.w) }
#[inline(always)]
pub fn ywyx(&self) -> Self { Self::new(self.y, self.w, self.y, self.x) }
#[inline(always)]
pub fn ywyy(&self) -> Self { Self::new(self.y, self.w, self.y, self.y) }
#[inline(always)]
pub fn ywyz(&self) -> Self { Self::new(self.y, self.w, self.y, self.z) }
#[inline(always)]
pub fn ywyw(&self) -> Self { Self::new(self.y, self.w, self.y, self.w) }
#[inline(always)]
pub fn ywzx(&self) -> Self { Self::new(self.y, self.w, self.z, self.x) }
#[inline(always)]
pub fn ywzy(&self) -> Self { Self::new(self.y, self.w, self.z, self.y) }
#[inline(always)]
pub fn ywzz(&self) -> Self { Self::new(self.y, self.w, self.z, self.z) }
#[inline(always)]
pub fn ywzw(&self) -> Self { Self::new(self.y, self.w, self.z, self.w) }
#[inline(always)]
pub fn ywwx(&self) -> Self { Self::new(self.y, self.w, self.w, self.x) }
#[inline(always)]
pub fn ywwy(&self) -> Self { Self::new(self.y, self.w, self.w, self.y) }
#[inline(always)]
pub fn ywwz(&self) -> Self { Self::new(self.y, self.w, self.w, self.z) }
#[inline(always)]
pub fn ywww(&self) -> Self { Self::new(self.y, self.w, self.w, self.w) }
#[inline(always)]
pub fn zxxx(&self) -> Self { Self::new(self.z, self.x, self.x, self.x) }
#[inline(always)]
pub fn zxxy(&self) -> Self { Self::new(self.z, self.x, self.x, self.y) }
#[inline(always)]
pub fn zxxz(&self) -> Self { Self::new(self.z, self.x, self.x, self.z) }
#[inline(always)]
pub fn zxxw(&self) -> Self { Self::new(self.z, self.x, self.x, self.w) }
#[inline(always)]
pub fn zxyx(&self) -> Self { Self::new(self.z, self.x, self.y, self.x) }
#[inline(always)]
pub fn zxyy(&self) -> Self { Self::new(self.z, self.x, self.y, self.y) }
#[inline(always)]
pub fn zxyz(&self) -> Self { Self::new(self.z, self.x, self.y, self.z) }
#[inline(always)]
pub fn zxyw(&self) -> Self { Self::new(self.z, self.x, self.y, self.w) }
#[inline(always)]
pub fn zxzx(&self) -> Self { Self::new(self.z, self.x, self.z, self.x) }
#[inline(always)]
pub fn zxzy(&self) -> Self { Self::new(self.z, self.x, self.z, self.y) }
#[inline(always)]
pub fn zxzz(&self) -> Self { Self::new(self.z, self.x, self.z, self.z) }
#[inline(always)]
pub fn zxzw(&self) -> Self { Self::new(self.z, self.x, self.z, self.w) }
#[inline(always)]
pub fn zxwx(&self) -> Self { Self::new(self.z, self.x, self.w, self.x) }
#[inline(always)]
pub fn zxwy(&self) -> Self { Self::new(self.z, self.x, self.w, self.y) }
#[inline(always)]
pub fn zxwz(&self) -> Self { Self::new(self.z, self.x, self.w, self.z) }
#[inline(always)]
pub fn zxww(&self) -> Self { Self::new(self.z, self.x, self.w, self.w) }
#[inline(always)]
pub fn zyxx(&self) -> Self { Self::new(self.z, self.y, self.x, self.x) }
#[inline(always)]
pub fn zyxy(&self) -> Self { Self::new(self.z, self.y, self.x, self.y) }
#[inline(always)]
pub fn zyxz(&self) -> Self { Self::new(self.z, self.y, self.x, self.z) }
#[inline(always)]
pub fn zyxw(&self) -> Self { Self::new(self.z, self.y, self.x, self.w) }
#[inline(always)]
pub fn zyyx(&self) -> Self { Self::new(self.z, self.y, self.y, self.x) }
#[inline(always)]
pub fn zyyy(&self) -> Self { Self::new(self.z, self.y, self.y, self.y) }
#[inline(always)]
pub fn zyyz(&self) -> Self { Self::new(self.z, self.y, self.y, self.z) }
#[inline(always)]
pub fn zyyw(&self) -> Self { Self::new(self.z, self.y, self.y, self.w) }
#[inline(always)]
pub fn zyzx(&self) -> Self { Self::new(self.z, self.y, self.z, self.x) }
#[inline(always)]
pub fn zyzy(&self) -> Self { Self::new(self.z, self.y, self.z, self.y) }
#[inline(always)]
pub fn zyzz(&self) -> Self { Self::new(self.z, self.y, self.z, self.z) }
#[inline(always)]
pub fn zyzw(&self) -> Self { Self::new(self.z, self.y, self.z, self.w) }
#[inline(always)]
pub fn zywx(&self) -> Self { Self::new(self.z, self.y, self.w, self.x) }
#[inline(always)]
pub fn zywy(&self) -> Self { Self::new(self.z, self.y, self.w, self.y) }
#[inline(always)]
pub fn zywz(&self) -> Self { Self::new(self.z, self.y, self.w, self.z) }
#[inline(always)]
pub fn zyww(&self) -> Self { Self::new(self.z, self.y, self.w, self.w) }
#[inline(always)]
pub fn zzxx(&self) -> Self { Self::new(self.z, self.z, self.x, self.x) }
#[inline(always)]
pub fn zzxy(&self) -> Self { Self::new(self.z, self.z, self.x, self.y) }
#[inline(always)]
pub fn zzxz(&self) -> Self { Self::new(self.z, self.z, self.x, self.z) }
#[inline(always)]
pub fn zzxw(&self) -> Self { Self::new(self.z, self.z, self.x, self.w) }
#[inline(always)]
pub fn zzyx(&self) -> Self { Self::new(self.z, self.z, self.y, self.x) }
#[inline(always)]
pub fn zzyy(&self) -> Self { Self::new(self.z, self.z, self.y, self.y) }
#[inline(always)]
pub fn zzyz(&self) -> Self { Self::new(self.z, self.z, self.y, self.z) }
#[inline(always)]
pub fn zzyw(&self) -> Self { Self::new(self.z, self.z, self.y, self.w) }
#[inline(always)]
pub fn zzzx(&self) -> Self { Self::new(self.z, self.z, self.z, self.x) }
#[inline(always)]
pub fn zzzy(&self) -> Self { Self::new(self.z, self.z, self.z, self.y) }
#[inline(always)]
pub fn zzzz(&self) -> Self { Self::new(self.z, self.z, self.z, self.z) }
#[inline(always)]
pub fn zzzw(&self) -> Self { Self::new(self.z, self.z, self.z, self.w) }
#[inline(always)]
pub fn zzwx(&self) -> Self { Self::new(self.z, self.z, self.w, self.x) }
#[inline(always)]
pub fn zzwy(&self) -> Self { Self::new(self.z, self.z, self.w, self.y) }
#[inline(always)]
pub fn zzwz(&self) -> Self { Self::new(self.z, self.z, self.w, self.z) }
#[inline(always)]
pub fn zzww(&self) -> Self { Self::new(self.z, self.z, self.w, self.w) }
#[inline(always)]
pub fn zwxx(&self) -> Self { Self::new(self.z, self.w, self.x, self.x) }
#[inline(always)]
pub fn zwxy(&self) -> Self { Self::new(self.z, self.w, self.x, self.y) }
#[inline(always)]
pub fn zwxz(&self) -> Self { Self::new(self.z, self.w, self.x, self.z) }
#[inline(always)]
pub fn zwxw(&self) -> Self { Self::new(self.z, self.w, self.x, self.w) }
#[inline(always)]
pub fn zwyx(&self) -> Self { Self::new(self.z, self.w, self.y, self.x) }
#[inline(always)]
pub fn zwyy(&self) -> Self { Self::new(self.z, self.w, self.y, self.y) }
#[inline(always)]
pub fn zwyz(&self) -> Self { Self::new(self.z, self.w, self.y, self.z) }
#[inline(always)]
pub fn zwyw(&self) -> Self { Self::new(self.z, self.w, self.y, self.w) }
#[inline(always)]
pub fn zwzx(&self) -> Self { Self::new(self.z, self.w, self.z, self.x) }
#[inline(always)]
pub fn zwzy(&self) -> Self { Self::new(self.z, self.w, self.z, self.y) }
#[inline(always)]
pub fn zwzz(&self) -> Self { Self::new(self.z, self.w, self.z, self.z) }
#[inline(always)]
pub fn zwzw(&self) -> Self { Self::new(self.z, self.w, self.z, self.w) }
#[inline(always)]
pub fn zwwx(&self) -> Self { Self::new(self.z, self.w, self.w, self.x) }
#[inline(always)]
pub fn zwwy(&self) -> Self { Self::new(self.z, self.w, self.w, self.y) }
#[inline(always)]
pub fn zwwz(&self) -> Self { Self::new(self.z, self.w, self.w, self.z) }
#[inline(always)]
pub fn zwww(&self) -> Self { Self::new(self.z, self.w, self.w, self.w) }
#[inline(always)]
pub fn wxxx(&self) -> Self { Self::new(self.w, self.x, self.x, self.x) }
#[inline(always)]
pub fn wxxy(&self) -> Self { Self::new(self.w, self.x, self.x, self.y) }
#[inline(always)]
pub fn wxxz(&self) -> Self { Self::new(self.w, self.x, self.x, self.z) }
#[inline(always)]
pub fn wxxw(&self) -> Self { Self::new(self.w, self.x, self.x, self.w) }
#[inline(always)]
pub fn wxyx(&self) -> Self { Self::new(self.w, self.x, self.y, self.x) }
#[inline(always)]
pub fn wxyy(&self) -> Self { Self::new(self.w, self.x, self.y, self.y) }
#[inline(always)]
pub fn wxyz(&self) -> Self { Self::new(self.w, self.x, self.y, self.z) }
#[inline(always)]
pub fn wxyw(&self) -> Self { Self::new(self.w, self.x, self.y, self.w) }
#[inline(always)]
pub fn wxzx(&self) -> Self { Self::new(self.w, self.x, self.z, self.x) }
#[inline(always)]
pub fn wxzy(&self) -> Self { Self::new(self.w, self.x, self.z, self.y) }
#[inline(always)]
pub fn wxzz(&self) -> Self { Self::new(self.w, self.x, self.z, self.z) }
#[inline(always)]
pub fn wxzw(&self) -> Self { Self::new(self.w, self.x, self.z, self.w) }
#[inline(always)]
pub fn wxwx(&self) -> Self { Self::new(self.w, self.x, self.w, self.x) }
#[inline(always)]
pub fn wxwy(&self) -> Self { Self::new(self.w, self.x, self.w, self.y) }
#[inline(always)]
pub fn wxwz(&self) -> Self { Self::new(self.w, self.x, self.w, self.z) }
#[inline(always)]
pub fn wxww(&self) -> Self { Self::new(self.w, self.x, self.w, self.w) }
#[inline(always)]
pub fn wyxx(&self) -> Self { Self::new(self.w, self.y, self.x, self.x) }
#[inline(always)]
pub fn wyxy(&self) -> Self { Self::new(self.w, self.y, self.x, self.y) }
#[inline(always)]
pub fn wyxz(&self) -> Self { Self::new(self.w, self.y, self.x, self.z) }
#[inline(always)]
pub fn wyxw(&self) -> Self { Self::new(self.w, self.y, self.x, self.w) }
#[inline(always)]
pub fn wyyx(&self) -> Self { Self::new(self.w, self.y, self.y, self.x) }
#[inline(always)]
pub fn wyyy(&self) -> Self { Self::new(self.w, self.y, self.y, self.y) }
#[inline(always)]
pub fn wyyz(&self) -> Self { Self::new(self.w, self.y, self.y, self.z) }
#[inline(always)]
pub fn wyyw(&self) -> Self { Self::new(self.w, self.y, self.y, self.w) }
#[inline(always)]
pub fn wyzx(&self) -> Self { Self::new(self.w, self.y, self.z, self.x) }
#[inline(always)]
pub fn wyzy(&self) -> Self { Self::new(self.w, self.y, self.z, self.y) }
#[inline(always)]
pub fn wyzz(&self) -> Self { Self::new(self.w, self.y, self.z, self.z) }
#[inline(always)]
pub fn wyzw(&self) -> Self { Self::new(self.w, self.y, self.z, self.w) }
#[inline(always)]
pub fn wywx(&self) -> Self { Self::new(self.w, self.y, self.w, self.x) }
#[inline(always)]
pub fn wywy(&self) -> Self { Self::new(self.w, self.y, self.w, self.y) }
#[inline(always)]
pub fn wywz(&self) -> Self { Self::new(self.w, self.y, self.w, self.z) }
#[inline(always)]
pub fn wyww(&self) -> Self { Self::new(self.w, self.y, self.w, self.w) }
#[inline(always)]
pub fn wzxx(&self) -> Self { Self::new(self.w, self.z, self.x, self.x) }
#[inline(always)]
pub fn wzxy(&self) -> Self { Self::new(self.w, self.z, self.x, self.y) }
#[inline(always)]
pub fn wzxz(&self) -> Self { Self::new(self.w, self.z, self.x, self.z) }
#[inline(always)]
pub fn wzxw(&self) -> Self { Self::new(self.w, self.z, self.x, self.w) }
#[inline(always)]
pub fn wzyx(&self) -> Self { Self::new(self.w, self.z, self.y, self.x) }
#[inline(always)]
pub fn wzyy(&self) -> Self { Self::new(self.w, self.z, self.y, self.y) }
#[inline(always)]
pub fn wzyz(&self) -> Self { Self::new(self.w, self.z, self.y, self.z) }
#[inline(always)]
pub fn wzyw(&self) -> Self { Self::new(self.w, self.z, self.y, self.w) }
#[inline(always)]
pub fn wzzx(&self) -> Self { Self::new(self.w, self.z, self.z, self.x) }
#[inline(always)]
pub fn wzzy(&self) -> Self { Self::new(self.w, self.z, self.z, self.y) }
#[inline(always)]
pub fn wzzz(&self) -> Self { Self::new(self.w, self.z, self.z, self.z) }
#[inline(always)]
pub fn wzzw(&self) -> Self { Self::new(self.w, self.z, self.z, self.w) }
#[inline(always)]
pub fn wzwx(&self) -> Self { Self::new(self.w, self.z, self.w, self.x) }
#[inline(always)]
pub fn wzwy(&self) -> Self { Self::new(self.w, self.z, self.w, self.y) }
#[inline(always)]
pub fn wzwz(&self) -> Self { Self::new(self.w, self.z, self.w, self.z) }
#[inline(always)]
pub fn wzww(&self) -> Self { Self::new(self.w, self.z, self.w, self.w) }
#[inline(always)]
pub fn wwxx(&self) -> Self { Self::new(self.w, self.w, self.x, self.x) }
#[inline(always)]
pub fn wwxy(&self) -> Self { Self::new(self.w, self.w, self.x, self.y) }
#[inline(always)]
pub fn wwxz(&self) -> Self { Self::new(self.w, self.w, self.x, self.z) }
#[inline(always)]
pub fn wwxw(&self) -> Self { Self::new(self.w, self.w, self.x, self.w) }
#[inline(always)]
pub fn wwyx(&self) -> Self { Self::new(self.w, self.w, self.y, self.x) }
#[inline(always)]
pub fn wwyy(&self) -> Self { Self::new(self.w, self.w, self.y, self.y) }
#[inline(always)]
pub fn wwyz(&self) -> Self { Self::new(self.w, self.w, self.y, self.z) }
#[inline(always)]
pub fn wwyw(&self) -> Self { Self::new(self.w, self.w, self.y, self.w) }
#[inline(always)]
pub fn wwzx(&self) -> Self { Self::new(self.w, self.w, self.z, self.x) }
#[inline(always)]
pub fn wwzy(&self) -> Self { Self::new(self.w, self.w, self.z, self.y) }
#[inline(always)]
pub fn wwzz(&self) -> Self { Self::new(self.w, self.w, self.z, self.z) }
#[inline(always)]
pub fn wwzw(&self) -> Self { Self::new(self.w, self.w, self.z, self.w) }
#[inline(always)]
pub fn wwwx(&self) -> Self { Self::new(self.w, self.w, self.w, self.x) }
#[inline(always)]
pub fn wwwy(&self) -> Self { Self::new(self.w, self.w, self.w, self.y) }
#[inline(always)]
pub fn wwwz(&self) -> Self { Self::new(self.w, self.w, self.w, self.z) }
#[inline(always)]
pub fn wwww(&self) -> Self { Self::new(self.w, self.w, self.w, self.w) }
}