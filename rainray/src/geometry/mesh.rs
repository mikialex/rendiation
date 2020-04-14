use crate::bvh::*;
use crate::math::*;

struct IndexMesh {
    index: Vec<u64>,
    position: Vec<f64>,
    normal: Vec<f64>,
}

impl IndexMesh {
    pub fn new(index: Vec<u64>, position: Vec<f64>, normal: Vec<f64>) -> IndexMesh {
        IndexMesh {
            index,
            position,
            normal,
        }
    }

    pub fn gen_primitive_list(&self) -> Vec<Primitive> {
        let face_count = self.index.len() / 3;
        let mut primitive_list: Vec<Primitive> = Vec::with_capacity(face_count);
        for (count_i, i) in (0..face_count).enumerate() {
            let position_a_index = self.index[i * 3] as usize;
            let position_b_index = self.index[i * 3 + 1] as usize;
            let position_c_index = self.index[i * 3 + 2] as usize;

            let point_a = Vec3::new(
                self.position[position_a_index],
                self.position[position_a_index + 1],
                self.position[position_a_index + 2],
            );

            let point_b = Vec3::new(
                self.position[position_b_index],
                self.position[position_b_index + 1],
                self.position[position_b_index + 2],
            );

            let point_c = Vec3::new(
                self.position[position_c_index],
                self.position[position_c_index + 1],
                self.position[position_c_index + 2],
            );

            let bounding_box = Box3::from_3points(&point_a, &point_b, &point_c);
            let center_point = bounding_box.center();
            primitive_list.push(Primitive {
                bounding_box,
                center_point,
                index: count_i as u64,
            });
        }
        primitive_list
    }
}

fn build_bvh_from_index_mesh(mesh: &IndexMesh) -> BVHAccel {
    let _primitive_list = mesh.gen_primitive_list();
    unimplemented!()
}
