use rendiation_math::Vec3;

pub struct Face3 {
    pub a: Vec3<f32>,
    pub b: Vec3<f32>,
    pub c: Vec3<f32>,
}

impl Face3 {
    pub fn new(
        a: Vec3<f32>,
        b: Vec3<f32>,
        c: Vec3<f32>,
    ) -> Self {
        Self {
            a, b, c
        }
    }

    pub fn barycentric(&self, p: Vec3<f32>) -> Vec3<f32>{
        let v0 = self.b-self.a;
        let v1 = self.c-self.a;
        let v2 = p-self.a;
        let d00 = v0.dot(v0);
        let d01 = v0.dot(v1);
        let d11 = v1.dot(v1);
        let d20 = v2.dot(v0);
        let d21 = v2.dot(v1);
        let denom = d00*d11-d01*d01;
        let v = (d11 * d20 - d01 * d21) / denom;
        let w = (d00 * d21 - d01 * d20) / denom;
        let u = 1.0 - v - w;
        Vec3::new(u,v,w)
    }
}