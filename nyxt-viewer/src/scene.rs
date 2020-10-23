pub struct SceneNodeDataWASM {
  node: SceneNodeData<GFX>,
  handle: usize,
  scene: Weak<RefCell<Scene<GFX>>>,
}
