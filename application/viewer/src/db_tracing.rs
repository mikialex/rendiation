#[derive(Debug)]
pub enum ViewerTracingEvent {
  Render,
}

impl database_tracing::TraceReplayTarget for ViewerTracingEvent {
  fn type_discriminant() -> u32 {
    10
  }
  fn is_replay_target(&self) -> bool {
    match self {
      ViewerTracingEvent::Render => true,
    }
  }
}

impl database_tracing::TraceIO for ViewerTracingEvent {
  fn write_len(&self) -> usize {
    1
  }

  fn write(&self, w: &mut impl std::io::prelude::Write) -> std::io::Result<usize> {
    match self {
      ViewerTracingEvent::Render => {
        w.write_all(&[0u8])?;
        Ok(1)
      }
    }
  }

  fn read(source: &mut dyn std::io::Read) -> std::io::Result<Self>
  where
    Self: Sized,
  {
    let mut tag = [0u8; 1];
    source.read_exact(&mut tag)?;
    match tag[0] {
      0 => Ok(ViewerTracingEvent::Render),
      other => Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("unknown ViewerTracingEvent tag: {}", other),
      )),
    }
  }
}
