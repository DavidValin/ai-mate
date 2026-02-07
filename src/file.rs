// ------------------------------------------------------------------
//  File utils
// ------------------------------------------------------------------



use std::path::PathBuf;

// PRIVATE
// ------------------------------------------------------------------

pub fn home_dir() -> Option<PathBuf> {
  if let Ok(h) = std::env::var("HOME") {
    if !h.trim().is_empty() {
      return Some(PathBuf::from(h));
    }
  }
  if let Ok(h) = std::env::var("USERPROFILE") {
    if !h.trim().is_empty() {
      return Some(PathBuf::from(h));
    }
  }
  let drive = std::env::var("HOMEDRIVE").ok();
  let path = std::env::var("HOMEPATH").ok();
  match (drive, path) {
    (Some(d), Some(p)) if !d.trim().is_empty() && !p.trim().is_empty() => {
      Some(PathBuf::from(format!("{d}{p}")))
    }
    _ => None,
  }
}