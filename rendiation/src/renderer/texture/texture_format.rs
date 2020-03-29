pub trait TextureFormat {
    type PixelDataType;
}

pub struct Rgba8UnormSrgb;

impl TextureFormat for Rgba8UnormSrgb {
    type PixelDataType = u32;
}