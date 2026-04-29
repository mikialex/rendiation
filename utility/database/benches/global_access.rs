use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use database::*;

fn criterion_benchmark(c: &mut Criterion) {
  setup_global_database(Default::default());

  declare_entity!(MyTestEntity);
  declare_component!(TestEntityFieldA, MyTestEntity, f32);
  declare_component!(TestEntityFieldA2, MyTestEntity, f32);
  declare_component!(TestEntityFieldA3, MyTestEntity, f32);
  declare_component!(TestEntityFieldA4, MyTestEntity, f32);
  declare_component!(TestEntityFieldA5, MyTestEntity, f32);
  declare_component!(TestEntityFieldA6, MyTestEntity, f32);

  global_database()
    .declare_entity::<MyTestEntity>()
    .declare_component::<TestEntityFieldA>()
    .declare_component::<TestEntityFieldA2>()
    .declare_component::<TestEntityFieldA3>()
    .declare_component::<TestEntityFieldA4>()
    .declare_component::<TestEntityFieldA5>()
    .declare_component::<TestEntityFieldA6>();

  let e = global_entity_of::<MyTestEntity>()
    .entity_writer()
    .new_entity(|w| w);

  c.bench_function("create entity creator from global", |b| {
    b.iter(|| {
      let writer = global_entity_of::<MyTestEntity>().entity_writer();
      black_box(writer);
    })
  });

  c.bench_function("read a value from global", |b| {
    b.iter(|| {
      let reader = read_global_db_component::<TestEntityFieldA>();
      let r = reader.get(e);
      black_box(r);
    })
  });

  c.bench_function("write a value from global", |b| {
    b.iter(|| {
      let mut writer = write_global_db_component::<TestEntityFieldA>();
      let r = writer.write(e, 1.);
      black_box(r);
    })
  });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
