impl<const USAGE: wgpu::BufferUsage> Buffer<USAGE> {
  pub fn new() -> Self {
    todo!()
  }
}

pub struct Buffer<const USAGE: wgpu::BufferUsage> {
  buffer: wgpu::Buffer,
}

pub struct If<const B: bool>;
pub trait True {}
impl True for If<true> {}

// https://internals.rust-lang.org/t/const-generics-where-restrictions/12742/4
impl<const USAGE: wgpu::BufferUsage> Buffer<USAGE>
where
  If<{ has_map_read(USAGE) }>: True,
{
  // only_impl_when_buffer_has_read_ability
  pub fn read(&self) -> Self {
    todo!()
  }
}

pub const fn has_map_read(usage: wgpu::BufferUsage) -> bool {
  usage.contains(wgpu::BufferUsage::MAP_READ)
}

//  a | b, const fn in trait not support yet(except primitive type), so let's use a const fn and transmute!
pub const fn or(a: wgpu::BufferUsage, b: wgpu::BufferUsage) -> wgpu::BufferUsage {
  unsafe { std::mem::transmute(a.bits() | b.bits()) }
}

#[test]
fn test() {
  const MAP_READ: wgpu::BufferUsage = wgpu::BufferUsage::MAP_READ;
  const INDEX: wgpu::BufferUsage = wgpu::BufferUsage::INDEX;
  const COPY_DST: wgpu::BufferUsage = wgpu::BufferUsage::COPY_DST;

  let buffer_a: Buffer<{ or(MAP_READ, INDEX) }> = todo!();
  let buffer_b: Buffer<{ or(or(MAP_READ, INDEX), COPY_DST) }> = todo!();
  let buffer_c: Buffer<{ INDEX }> = todo!();

  // compile
  buffer_a.read();
  buffer_b.read();

  // not compile
  // buffer_c.read();
}
