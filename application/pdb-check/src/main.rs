use std::{collections::HashSet, fs::File};

use pdb::FallibleIterator;

fn main() -> pdb::Result<()> {
  let path = std::env::current_dir()
    .unwrap()
    .join("./target/debug/viewer.pdb");
  let file = File::open(path)?;
  println!(
    "full file size: {:3?} mb",
    file.metadata().unwrap().len() as f32 / 1024. / 1024.
  );

  let mut pdb = pdb::PDB::open(file)?;

  let symbol_table = pdb.global_symbols()?;
  let ty_info = pdb.type_information()?;

  let mut all_name_byte_sum = 0;

  // this is useless actually, no duplications ever found
  let mut deduplicate = HashSet::new();

  let mut tys = ty_info.iter();
  while let Some(ty) = tys.next()? {
    if let Ok(ty) = ty.parse() {
      if let Some(name) = ty.name() {
        if deduplicate.insert(name.as_bytes().as_ptr() as u64) {
          all_name_byte_sum += name.len();
        }
        // if name.len() > 50 * 1024 {
        //   println!("very long name: {}", name);
        // }
      }
    }
    //
  }

  let mut symbols = symbol_table.iter();
  while let Some(symbol) = symbols.next()? {
    if let Ok(symbol) = symbol.parse() {
      if let Some(name) = symbol.name() {
        if deduplicate.insert(name.as_bytes().as_ptr() as u64) {
          all_name_byte_sum += name.len();
        }
        // if name.len() > 10 * 1024 {
        //   println!("very long name: {}", name.len());
        // }
      }
    }
  }

  println!(
    "all name byte sum: {:3?} mb",
    all_name_byte_sum as f32 / 1024. / 1024.
  );

  Ok(())
}
