use rendiation_math::*;

#[derive(Debug, Copy, Clone)]
pub struct Plane{
    pub normal: Vec3<f32>,
    pub constant: f32,
}

impl Plane{
    pub fn new(normal: Vec3<f32>, constant: f32)-> Self{
        Plane{
            normal,constant
        }
    }

    pub fn distance_to_point(&self, point: Vec3<f32>) ->f32{
		self.normal.dot(point) + self.constant
    }

    pub fn set_components(&mut self, x: f32,y: f32,z: f32,w: f32) -> &mut Self {
        self.normal.set(x, y, z);
        self.constant =w;
        self
    }

    pub fn normalize(&mut self) -> &mut Self {
        let inverse_normal_length = 1.0 / self.normal.length();
		self.normal *= inverse_normal_length;
		self.constant *= inverse_normal_length;
        self
    }
}
