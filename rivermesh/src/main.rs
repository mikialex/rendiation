
use std::{path::Path};

pub mod half_edge_mesh;
pub mod simplification;

use half_edge_mesh::*;

fn main() {

    let bunny = tobj::load_obj(&Path::new("assets/bunny.obj"));
    assert!(bunny.is_ok());
    let (models, materials) = bunny.unwrap();

    println!("# of models: {}", models.len());
    println!("# of materials: {}", materials.len());
    for (i, m) in models.iter().enumerate() {
        let mesh = &m.mesh;
        println!("model[{}].name = \'{}\'", i, m.name);
        println!("model[{}].mesh.material_id = {:?}", i, mesh.material_id);

        // Normals and texture coordinates are also loaded, but not printed in this example
        println!("model[{}].vertices: {}", i, mesh.positions.len() / 3);
        assert!(mesh.positions.len() % 3 == 0);

        let _mesh = HalfEdgeMesh::from_buffer(&mesh.positions, &mesh.indices);
        let _a = 1;
    }
    let _b = 1;
}
