use crate::*;

pub struct DatabaseMutationWatch {
  component_changes: FastHashMap<TypeId, Box<dyn Any>>,
  db: Database,
}

impl DatabaseMutationWatch {
  pub fn new(db: &Database) -> Self {
    Self {
      component_changes: Default::default(),
      db: db.clone(),
    }
  }

  pub fn watch<C: ComponentSemantic>(
    &self,
  ) -> impl ReactiveCollection<AllocIdx<C::Entity>, C::Data> {
    self.db.access_ecg::<C::Entity, _>(|e| {
      e.access_component::<C, _>(|c| {
        c.group_watchers.on(|change| {
          //
          false
        })
        //
      })
    });
    //
  }
}
