// ------------------------------------------------------------------
//  STT - Speech to Text
// ------------------------------------------------------------------



use std::sync::{OnceLock};
use std::time::{Instant};
use std::path::PathBuf;
use std::process::Command;

// API
// ------------------------------------------------------------------

pub fn default_whisper_model_path() -> String {
  let fallback = ".whisper-models/ggml-large-v3-q5_0.bin";
  if let Some(home) = crate::file::home_dir() {
    return home.join(fallback).to_string_lossy().to_string();
  }
  // Last resort: relative path.
  fallback.to_string()
}


pub fn warm_up_whisper(
  start_instant:&OnceLock<Instant>,
  args: &crate::config::Args
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  // Resolve early so we fail fast with a clear error if Whisper isn't installed.
  let whisper_bin = resolve_whisper_program(args)?;

  crate::log::log("info", &format!("Whisper binary: {}", whisper_bin.to_string_lossy()));
  crate::log::log("info", "Warming up Whisper model...");

  // A short silence chunk is enough to force model load / init.
  let silence = crate::audio::AudioChunk {
    data: vec![0.0; 16_000 / 2], // ~0.5s at 16kHz
    channels: 1,
    sample_rate: 16_000,
  };

  // We don't care what the transcription is; we just want to pay the one-time init cost upfront.
  let _ = whisper_transcribe(&start_instant, &silence, args)?;

  crate::log::log("info", "Whisper warm-up complete.");

  Ok(())
}


pub fn whisper_transcribe(
   start_instant:&OnceLock<Instant>,
  utt: &crate::audio::AudioChunk,
  args: &crate::config::Args,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
  let wav_path = crate::audio::write_tmp_wav_16k_mono(start_instant, utt)?;

  // Equivalent to the old whisper-wrapper.sh:
  //   whisper-cli -m <MODEL> -np -nt -f <WAV>
  let whisper_bin = resolve_whisper_program(args)?;
  let wav_s = wav_path.to_string_lossy().to_string();
  let out = Command::new(&whisper_bin)
    .args([
      "-m",
      args.whisper_model_path.as_str(),
      "-np",
      "-nt",
      "--language",
      args.language.as_str(),
      "-f",
      wav_s.as_str(),
    ])
    .output()?;

  if !out.status.success() {
    let stderr = String::from_utf8_lossy(&out.stderr);
    return Err(format!("Whisper command failed: {stderr}").into());
  }

  // Remove newlines in Rust so it works cross-platform (Linux/macOS/Windows).
  let stdout = String::from_utf8_lossy(&out.stdout).to_string();
  let cleaned = stdout.replace(['\r', '\n'], "");
  Ok(cleaned.trim().to_string())
}

// PRIVATE
// ------------------------------------------------------------------

fn find_in_path(program: &str) -> Option<PathBuf> {
  let path_var = std::env::var_os("PATH")?;
  let paths = std::env::split_paths(&path_var);

  // On Windows, PATHEXT defines executable extensions.
  let exts: Vec<String> = if cfg!(windows) {
    std::env::var("PATHEXT")
      .ok()
      .map(|v| {
        v.split(';')
          .map(|s| s.trim().to_string())
          .filter(|s| !s.is_empty())
          .collect()
      })
      .unwrap_or_else(|| vec![".EXE".into(), ".CMD".into(), ".BAT".into()])
  } else {
    vec!["".into()]
  };

  for dir in paths {
    for ext in &exts {
      let candidate = dir.join(format!("{program}{ext}"));
      if candidate.is_file() {
        return Some(candidate);
      }
    }
  }
  None
}


fn resolve_whisper_program(
  args: &crate::config::Args,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
  if let Some(cmd) = args
    .whisper_cmd
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
  {
    let p = PathBuf::from(cmd);
    if p.components().count() > 1 {
      if p.is_file() {
        return Ok(p);
      }
      return Err(format!("WHISPER_CMD points to a non-existent file: {cmd}").into());
    }

    if let Some(found) = find_in_path(cmd) {
      return Ok(found);
    }
    return Err(format!("Whisper command '{cmd}' not found in PATH").into());
  }

  if let Some(found) = find_in_path("whisper-cli") {
    return Ok(found);
  }
  if let Some(found) = find_in_path("whisper") {
    return Ok(found);
  }

  Err("Could not find a Whisper CLI. Install 'whisper-cli' (preferred) or 'whisper' and ensure it is in PATH.".into())
}
