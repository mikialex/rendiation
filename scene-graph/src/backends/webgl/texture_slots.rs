pub struct TextureSlotStates{
    slots: Vec<TextureSlot>
}

pub enum WebGLTextureBindType{
    Texture2D = 0x0DE1, // todo use webglctx const
    TextureCubeMap = 0x8513,
}

pub struct TextureSlotBindInfo{
    bind_type: WebGLTextureBindType,
    // texture_handle: Handle<>
}