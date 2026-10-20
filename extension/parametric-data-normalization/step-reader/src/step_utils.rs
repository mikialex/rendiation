use std::fs;
use std::path::Path;

/// Fix common non-standard formatting that `ruststep`'s strict tokenizer rejects:
/// - `ISO - 10303 - 21;` → `ISO-10303-21;` (spaces in the magic header)
/// - `# 123 =` → `#123=` (spaces between `#` and entity ID)
pub fn normalize_step(raw: &str) -> String {
  let mut lines: Vec<&str> = raw.lines().collect();

  // Fix magic header on first line
  if let Some(first) = lines.first() {
    if first.starts_with("ISO") && first.contains("10303") && first.contains("21") {
      lines[0] = "ISO-10303-21;";
    }
  }

  // Fix magic footer on last line
  if let Some(last) = lines.last() {
    if last.contains("END") && last.contains("ISO") && last.contains("10303") {
      let idx = lines.len() - 1;
      lines[idx] = "END-ISO-10303-21;";
    }
  }

  lines
    .join("\n")
    .replace("\n# ", "\n#")
    .replace("\r\n# ", "\r\n#")
}

/// Check whether a path has a `.stp` or `.step` extension.
pub fn is_stp_file(path: &Path) -> bool {
  path
    .extension()
    .map(|e| e.eq_ignore_ascii_case("stp") || e.eq_ignore_ascii_case("step"))
    .unwrap_or(false)
}

/// Recursively visit all `.stp`/`.step` files under a directory.
pub fn visit_stp_files(dir: &Path, f: &mut dyn FnMut(&Path)) {
  let entries = match fs::read_dir(dir) {
    Ok(e) => e,
    Err(_) => return,
  };

  for entry in entries.flatten() {
    let path = entry.path();
    if path.is_dir() {
      visit_stp_files(&path, f);
    } else if is_stp_file(&path) {
      f(&path);
    }
  }
}
