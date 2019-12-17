use crate::sphere::Sphere;
use crate::plane::Plane;
use rendiation_math::*;

pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum{
    pub fn new() -> Self {
        Self {
            planes: [Plane::new(Vec3::new(1.0, 1., 1.), 1.);6]
        }
    }

    pub fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        let neg_radius = - sphere.radius;
        
        for p in &self.planes {
            let distance = p.distance_to_point( sphere.center );
			if distance < neg_radius {
				return false;
			}
        };

        true
    }

    pub fn set_from_matrix(&mut self, m: Mat4<f32>) -> &Self{
		self.planes[ 0 ].set_components( m.a4 - m.a1, m.b4 - m.b1, m.c4 - m.c1, m.d4 - m.d1 ).normalize();
		self.planes[ 1 ].set_components( m.a4 + m.a1, m.b4 + m.b1, m.c4 + m.c1, m.d4 + m.d1 ).normalize();
		self.planes[ 2 ].set_components( m.a4 + m.a2, m.b4 + m.b2, m.c4 + m.c2, m.d4 + m.d2 ).normalize();
		self.planes[ 3 ].set_components( m.a4 - m.a2, m.b4 - m.b2, m.c4 - m.c2, m.d4 - m.d2 ).normalize();
		self.planes[ 4 ].set_components( m.a4 - m.a3, m.b4 - m.b3, m.c4 - m.c3, m.d4 - m.d3 ).normalize();
		self.planes[ 5 ].set_components( m.a4 + m.a3, m.b4 + m.b3, m.c4 + m.c3, m.d4 + m.d3 ).normalize();

        self
    }
}