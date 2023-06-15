use anyhow::Result;
use ash::Entry;
use rendiation_vulkan::*;

fn main() -> Result<()> {
  let entry = Entry::linked();

  let vk_version = Version {
    major: 1,
    minor: 3,
    ..Default::default()
  };

  let mut _instance = Instance::new(&entry, None, vk_version, "test_vk_app")?;

  Ok(())
}
