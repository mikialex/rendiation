use crate::*;

/// The ffi view of a database, built by reflecting all registered entities and components.
pub struct FfiSchema {
  pub entities: Vec<FfiEntity>,
  pub components: Vec<FfiComponent>,
  /// components whose data type can not be mapped to C yet
  pub unsupported: Vec<FfiUnsupportedComponent>,
  pub hash: u64,
  entity_by_key: FastHashMap<u64, usize>,
  component_by_key: FastHashMap<u64, usize>,
}

pub struct FfiEntity {
  pub key: u64,
  pub name: String,
  pub c_name: String,
  pub id: EntityId,
}

pub struct FfiComponent {
  pub key: u64,
  pub name: String,
  pub c_name: String,
  pub entity: EntityId,
  pub id: ComponentId,
  pub foreign_key: Option<EntityId>,
  pub shape: &'static Shape,
  pub value: CType,
  /// the read value contains pointers that are kept alive by the [FfiArena]
  pub has_heap_data: bool,
}

pub struct FfiUnsupportedComponent {
  pub name: String,
  pub reason: String,
}

impl FfiSchema {
  pub fn from_database(db: &Database) -> Self {
    let mut tables: Vec<_> = db
      .tables
      .read()
      .iter()
      .map(|(id, t)| (*id, t.clone()))
      .collect();
    tables.sort_by(|a, b| a.1.name().cmp(b.1.name()));

    let mut entities = Vec::new();
    let mut raw_components = Vec::new();
    for (id, table) in &tables {
      entities.push(FfiEntity {
        key: stable_hash(table.name()),
        name: table.name().to_string(),
        c_name: sanitize_ident(table.short_name()),
        id: *id,
      });
      table.visit_components(|c| {
        raw_components.push((
          c.name.clone(),
          c.short_name.clone(),
          c.entity_type_id,
          c.component_type_id,
          c.as_foreign_key,
          c.data_meta.shape,
        ))
      });
    }
    dedup_c_names(
      entities
        .iter_mut()
        .map(|e| (e.name.as_str(), &mut e.c_name)),
    );
    raw_components.sort_by(|a, b| a.0.cmp(&b.0));

    let mut builder = CTypeBuilder::default();
    let mut components = Vec::new();
    let mut unsupported = Vec::new();
    for (name, short_name, entity, id, foreign_key, shape) in raw_components {
      match builder.build(shape) {
        Ok(value) => components.push(FfiComponent {
          key: stable_hash(&name),
          c_name: sanitize_ident(&short_name),
          name,
          entity,
          id,
          foreign_key,
          shape,
          has_heap_data: value.has_heap_data(),
          value,
        }),
        Err(reason) => unsupported.push(FfiUnsupportedComponent {
          name,
          reason: reason.0,
        }),
      }
    }
    dedup_c_names(
      components
        .iter_mut()
        .map(|c| (c.name.as_str(), &mut c.c_name)),
    );

    let entity_by_key = build_key_map(entities.iter().map(|e| (e.key, &e.name)));
    let component_by_key = build_key_map(components.iter().map(|c| (c.key, &c.name)));

    let mut signature = String::new();
    for e in &entities {
      write!(signature, "E{}={};", e.name, e.key).unwrap();
    }
    for c in &components {
      write!(signature, "C{}={}:", c.name, c.key).unwrap();
      c.value.write_signature(&mut signature);
      signature.push(';');
    }

    Self {
      hash: stable_hash(&signature),
      entities,
      components,
      unsupported,
      entity_by_key,
      component_by_key,
    }
  }

  pub fn entity(&self, key: u64) -> Option<&FfiEntity> {
    self.entity_by_key.get(&key).map(|i| &self.entities[*i])
  }

  pub fn entity_by_id(&self, id: EntityId) -> Option<&FfiEntity> {
    self.entities.iter().find(|e| e.id == id)
  }

  pub fn component(&self, key: u64) -> Option<&FfiComponent> {
    self
      .component_by_key
      .get(&key)
      .map(|i| &self.components[*i])
  }

  pub fn create_entity(&self, db: &Database, entity: u64) -> Option<RawEntityHandle> {
    let entity = self.entity(entity)?;
    Some(db.entity_writer_untyped_dyn(entity.id).new_entity(|w| w))
  }

  pub fn delete_entity(&self, db: &Database, entity: u64, handle: RawEntityHandle) -> FfiStatus {
    let Some(entity) = self.entity(entity) else {
      return FfiStatus::UnknownKey;
    };
    let mut writer = db.entity_writer_untyped_dyn(entity.id);
    if !writer.is_alive(handle) {
      return FfiStatus::DeadHandle;
    }
    writer.delete_entity(handle);
    FfiStatus::Ok
  }

  /// # Safety
  ///
  /// `src` must point to a readable and aligned C value of the component.
  pub unsafe fn write_component(
    &self,
    db: &Database,
    component: u64,
    handle: RawEntityHandle,
    src: *const u8,
  ) -> FfiStatus {
    let Some(c) = self.component(component) else {
      return FfiStatus::UnknownKey;
    };
    let written = unsafe {
      with_rust_value_from_c(c.shape, &c.value, src, |ptr| {
        db.access_table_dyn(c.entity, |t| {
          t.access_component(c.id, |com| com.write_single_checked(handle, ptr))
        })
      })
    };
    match written {
      Ok(Some(true)) => FfiStatus::Ok,
      Ok(Some(false)) => FfiStatus::DeadHandle,
      Ok(None) => FfiStatus::UnknownKey,
      Err(e) => {
        log::error!("failed to write {} from C: {e}", c.name);
        FfiStatus::InvalidValue
      }
    }
  }

  /// The heap parts of the read value are allocated in `arena`.
  ///
  /// # Safety
  ///
  /// `dst` must point to a writable and aligned C value of the component.
  pub unsafe fn read_component(
    &self,
    db: &Database,
    component: u64,
    handle: RawEntityHandle,
    dst: *mut u8,
    arena: &mut FfiArena,
  ) -> FfiStatus {
    let Some(c) = self.component(component) else {
      return FfiStatus::UnknownKey;
    };
    let read = db.access_table_dyn(c.entity, |t| {
      t.access_component(c.id, |com| {
        let view = com.read_untyped();
        let ptr = view.get(handle)?;
        Some(unsafe { write_rust_value_to_c(c.shape, &c.value, ptr, dst, arena) })
      })
    });
    match read {
      Some(Some(Ok(()))) => FfiStatus::Ok,
      Some(Some(Err(e))) => {
        log::error!("failed to read {} into C: {e}", c.name);
        FfiStatus::InvalidValue
      }
      Some(None) => FfiStatus::DeadHandle,
      None => FfiStatus::UnknownKey,
    }
  }
}

fn build_key_map<'a>(items: impl Iterator<Item = (u64, &'a String)>) -> FastHashMap<u64, usize> {
  let mut map = FastHashMap::default();
  let mut names: FastHashMap<u64, &String> = FastHashMap::default();
  for (index, (key, name)) in items.enumerate() {
    if let Some(previous) = names.insert(key, name) {
      panic!("ffi key collision between {previous} and {name}");
    }
    map.insert(key, index);
  }
  map
}

/// The short names may collide, fallback to the full name for the collided ones.
fn dedup_c_names<'a>(items: impl Iterator<Item = (&'a str, &'a mut String)>) {
  let items: Vec<_> = items.collect();
  let mut count: FastHashMap<String, usize> = FastHashMap::default();
  for (_, c_name) in &items {
    *count.entry((*c_name).clone()).or_default() += 1;
  }
  for (name, c_name) in items {
    if count[c_name.as_str()] > 1 {
      *c_name = sanitize_ident(name);
    }
  }
}
