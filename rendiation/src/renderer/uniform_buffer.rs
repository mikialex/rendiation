pub trait UniformBuffer {
    fn update(renderer: WGPURenderer){

    }
}

struct CameraUBO {
    projection_matrix: Mat4<f32>,
    model_matrix: Mat4<f32>,
    model_view_matrix: Mat4<f32>,
}

struct CameraUBODerived {
    projection_matrix: Mat4<f32>,
    model_matrix: Mat4<f32>,
    model_view_matrix: Mat4<f32>,
    buffer: WGPUBuffer
}

struct UBODescriptor {
    
}