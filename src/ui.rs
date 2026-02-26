// ------------------------------------------------------------------
//  UI
// ------------------------------------------------------------------

use crate::log;
use crate::state::{get_speed, get_voice, GLOBAL_STATE};
use crossbeam_channel::Receiver;
use crossterm::{
  cursor::{Hide, MoveTo},
  execute,
  style::{Print, ResetColor},
  terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};

fn adjust_scroll(previous_lines_buffer: &[String]) -> i32 {
  let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
  let max_display = if log::is_verbose() {
    ((terminal_height as f32) / 1.6).round() as usize
  } else {
    terminal_height as usize - 2
  };
  let new_offset = previous_lines_buffer.len().saturating_sub(max_display);
  if let Some(state) = GLOBAL_STATE.get() {
    let current = state
      .scroll_offset
      .load(std::sync::atomic::Ordering::Relaxed) as usize;
    // If user has scrolled up (current < new_offset), keep current; otherwise move to bottom
    let adjusted = if current < new_offset {
      new_offset
    } else {
      current
    };
    state
      .scroll_offset
      .store(adjusted as i32, std::sync::atomic::Ordering::Relaxed);
  }

  new_offset as i32
}

use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc, Mutex,
};
use std::thread;
use std::time::Duration;

// API

pub static STOP_STREAM: AtomicBool = AtomicBool::new(false);

pub static UNFINISHED_LAST_LINE_BUFFER: Mutex<String> = Mutex::new(String::new());
// ------------------------------------------------------------------

// ANSI label styling
pub const USER_LABEL: &str = "\x1b[47;30mUSER:\x1b[0m"; // white bg, black text
pub const ASSIST_LABEL: &str = "\x1b[48;5;22;37mASSISTANT:\x1b[0m"; // dark green bg, white text

pub fn spawn_ui_thread(
  ui: crate::state::UiState,
  stop_all_rx: Receiver<()>,
  status_line: Arc<Mutex<String>>,
  peak: Arc<Mutex<f32>>,
  ui_rx: Receiver<String>,
) -> thread::JoinHandle<()> {
  // separate thread for bottom bar update + render
  thread::spawn(move || {
    let mut out = io::stdout();
    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut i = 0usize;

    let ui_for_bg = ui.clone();
    let status_line_for_bg = status_line.clone();
    let peak_for_bg = peak.clone();

    let mut out_for_closure = io::stdout();
    let ui_rx_for_closure = ui_rx.clone();
    let stop_all_rx_for_closure = stop_all_rx.clone();
    thread::spawn(move || {
      let mut i_for_bg = i;
      loop {
        i_for_bg = (i_for_bg + 1) % spinner.len();
        let full_bar = update_bottom_bar(
          &ui_for_bg,
          &status_line_for_bg,
          &peak_for_bg,
          &spinner,
          &mut i_for_bg,
        );
        print_bottom_bar(&mut out_for_closure, &full_bar).unwrap();

        if stop_all_rx_for_closure.try_recv().is_ok() {
          while let Ok(_) = ui_rx_for_closure.try_recv() {}
        }
        thread::sleep(Duration::from_millis(35));
      }
    });

    // hide cursor
    execute!(out, Hide).unwrap();

    // previous_lines_buffer for top region
    let mut top_lines: Vec<String> = Vec::new();
    let mut exit_ui = false;
    let mut prev_scroll = 0usize;
    let mut needs_full_redraw = false;

    loop {
      if stop_all_rx.try_recv().is_ok() {
        while let Ok(_) = ui_rx.try_recv() {}
        break;
      }
      let state = GLOBAL_STATE.get().expect("AppState not initialized");
      let conversation_paused = state.conversation_paused.load(Ordering::Relaxed);
      let cur_scroll = GLOBAL_STATE
        .get()
        .unwrap()
        .scroll_offset
        .load(Ordering::Relaxed) as usize;

      let (cols_raw, terminal_height) = terminal::size().unwrap_or((80, 24));
      let cols = cols_raw as usize;
      // cur_scroll will be read each loop iteration

      while let Ok(msg) = ui_rx.try_recv() {
        if stop_all_rx.try_recv().is_ok() {
          while let Ok(_) = ui_rx.try_recv() {}
          break;
        }
        // Check if scroll offset changed without a message
        if cur_scroll != prev_scroll {
          redraw_top_region(&mut out, &top_lines, terminal_height, &[], true);
          needs_full_redraw = true;
        }

        let mut parts = msg.splitn(2, '|');
        let msg_type = parts.next().unwrap_or("");
        let msg_str = parts.next().unwrap_or(msg.as_str());

        if !conversation_paused {
          match msg_type {
            "line" => {
              let (label, body) = if msg_str.starts_with(USER_LABEL) {
                (
                  USER_LABEL,
                  msg_str.strip_prefix(USER_LABEL).unwrap_or("").trim(),
                )
              } else if msg_str.starts_with(ASSIST_LABEL) {
                (
                  ASSIST_LABEL,
                  msg_str.strip_prefix(ASSIST_LABEL).unwrap_or("").trim(),
                )
              } else {
                ("", msg_str)
              };

              if !label.is_empty() {
                print_line(&mut top_lines, label);
              }

              if !body.is_empty() {
                print_inline_chunk(&mut out, &mut top_lines, body, cols);
              }
            }

            "stream" => {
              // Skip if stream rendering is paused but reset flag
              if STOP_STREAM.load(Ordering::Relaxed) {
                break;
              }
              print_inline_chunk(&mut out, &mut top_lines, msg_str, cols);
            }
            "stop_ui" => {
              print_line(&mut top_lines, "");
              print_line(&mut top_lines, "🛑 USER interrupted");
              print_line(&mut top_lines, "");
              while let Ok(_) = ui_rx.try_recv() {}
              exit_ui = true;
              STOP_STREAM.store(true, Ordering::Relaxed);
              break;
            }
            _ => {}
          }
        }
        if needs_full_redraw {
          redraw_top_region(&mut out, &top_lines, terminal_height, &[], true);
          needs_full_redraw = false;
        }

        let full_bar = update_bottom_bar(&ui, &status_line, &peak, &spinner, &mut i);
        print_bottom_bar(&mut out, &full_bar).unwrap();
      }

      if exit_ui {
        // Reset flags and clear stream stop
        STOP_STREAM.store(false, Ordering::Relaxed);
        exit_ui = false;
        continue;
      }
      // redraw if scroll changed without new message
      if cur_scroll != prev_scroll {
        redraw_top_region(&mut out, &top_lines, terminal_height, &[], true);
      }
      // set prev_scroll for next loop
      prev_scroll = cur_scroll;
    }
  })
}

// PRIVATE
// ------------------------------------------------------------------

// delay per character for smooth typing
const STREAM_DELAY_MS: u64 = 2;

fn print_line(previous_lines_buffer: &mut Vec<String>, line: &str) {
  // Flush any unfinished buffer content before printing a new line
  let mut buf_guard = UNFINISHED_LAST_LINE_BUFFER.lock().unwrap();
  if !buf_guard.is_empty() {
    previous_lines_buffer.push((*buf_guard).clone());
    buf_guard.clear();
    adjust_scroll(previous_lines_buffer);
  }
  previous_lines_buffer.push(line.to_string());

  // render the line immediately
  if previous_lines_buffer.len() >= 2 {
    let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
    let line_no = (previous_lines_buffer.len() - 1) as u16; // index of the line just added
    let line_to_print = &previous_lines_buffer[previous_lines_buffer.len() - 1];
    let mut out = io::stdout();
    execute!(
      out,
      MoveTo(0, line_no),
      Clear(ClearType::CurrentLine),
      Print(line_to_print)
    )
    .unwrap();
    out.flush().unwrap();

    // Update scroll offset if previous_lines_buffer exceeds visible area
    adjust_scroll(previous_lines_buffer);
    // redraw top region after any scroll adjustment
    redraw_top_region(&mut out, &previous_lines_buffer, terminal_height, &[], true);
  }
}

fn print_inline_chunk<W: Write>(
  out: &mut W,
  previous_lines_buffer: &mut Vec<String>,
  chunk: &str,
  cols: usize,
) {
  // Ensure there is at least one line
  if previous_lines_buffer.is_empty() {
    previous_lines_buffer.push(String::new());
    adjust_scroll(&previous_lines_buffer);
    let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
    redraw_top_region(out, &previous_lines_buffer, terminal_height, &[], true);
  }

  for ch in chunk.chars() {
    // Stop early if interrupted
    if STOP_STREAM.load(Ordering::Relaxed) {
      STOP_STREAM.store(false, Ordering::Relaxed);
      return;
    }

    // Current unfinished buffer
    let mut buf_guard = UNFINISHED_LAST_LINE_BUFFER.lock().unwrap();
    let unfinished_len = get_visible_len_for(&buf_guard);

    // Check if adding this character would exceed width
    if unfinished_len + 1 > cols {
      // Wrap: flush current buffer as a line, but keep last word on new line if possible
      if !buf_guard.is_empty() {
        let buf_string = (*buf_guard).clone();
        // Find last whitespace to avoid breaking a word
        if let Some(idx) = buf_string.rfind(|c: char| c.is_whitespace()) {
          let (first, second) = buf_string.split_at(idx + 1);
          previous_lines_buffer.push(first.to_string());
          *buf_guard = second.to_string();
          // Render the wrapped line immediately
          let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
          redraw_top_region(out, &previous_lines_buffer, terminal_height, &[], true);
        } else {
          // No whitespace, push whole buffer and start new line
          previous_lines_buffer.push(buf_string);
          buf_guard.clear();
        }
        adjust_scroll(&previous_lines_buffer);
      }

      // Add current character to the new line buffer
      buf_guard.push(ch);
      // Render the new line
      let current_line = (*buf_guard).clone();
      let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
      let max_line = terminal_height - 2;
      let mut line_no = previous_lines_buffer.len() as u16;
      if line_no > max_line {
        line_no = max_line;
      }
      execute!(
        out,
        MoveTo(0, line_no),
        Clear(ClearType::CurrentLine),
        Print(&current_line)
      )
      .unwrap();
      if line_no == max_line {
        redraw_top_region(out, &previous_lines_buffer, terminal_height, &[], true);
      }
    } else if ch == '\n'
      || (ch == '.'
        && !buf_guard.is_empty()
        && buf_guard.chars().last().unwrap_or(' ').is_ascii_digit())
    {
      if ch == '.' {
        // Append dot to buffer but do not break line
        // (avoids adding new line if dot is right after a number)
        buf_guard.push(ch);
        continue;
      }
      // Handle newline or other breaks
      if !buf_guard.is_empty() {
        let buf_string = (*buf_guard).clone();
        previous_lines_buffer.push(buf_string);
        buf_guard.clear();
        adjust_scroll(&previous_lines_buffer);
      }
      // No empty line added
    } else {
      buf_guard.push(ch);
      let current_line = (*buf_guard).clone();
      let (_, terminal_height) = terminal::size().unwrap_or((80, 24));
      let max_line = terminal_height - 2;
      let mut line_no = previous_lines_buffer.len() as u16;
      if line_no > max_line {
        line_no = max_line;
      }
      execute!(
        out,
        MoveTo(0, line_no),
        Clear(ClearType::CurrentLine),
        Print(&current_line)
      )
      .unwrap();
      if line_no == max_line {
        redraw_top_region(out, &previous_lines_buffer, terminal_height, &[], true);
      }
    }

    thread::sleep(Duration::from_millis(STREAM_DELAY_MS));
  }
}

fn redraw_top_region<W: Write>(
  out: &mut W,
  previous_lines_buffer: &[String],
  max_height: u16,
  prev_buffer: &[String],
  full_redraw: bool,
) {
  let draw_height = if log::is_verbose() {
    // keep space in verbose mode to see the logs
    ((max_height as f32) / 1.6).round() as usize
  } else {
    max_height.saturating_sub(2) as usize // leave 2 lines space for error logs
  };

  // Determine the start line so the bottom of the previous_lines_buffer is visible
  let max_scroll = previous_lines_buffer.len().saturating_sub(draw_height);
  let scroll_raw = GLOBAL_STATE
    .get()
    .unwrap()
    .scroll_offset
    .load(std::sync::atomic::Ordering::Relaxed);
  let scroll = if scroll_raw < 0 {
    0
  } else {
    scroll_raw as usize
  };

  // only display visible portion and keep last two lines empty
  let effective_len = previous_lines_buffer.len();

  let start = scroll;

  for i in 0..draw_height {
    let idx = start + i;
    if idx < effective_len {
      let line = &previous_lines_buffer[idx];
      let prev_line = prev_buffer.get(idx).map(|s| s.as_str()).unwrap_or("");
      if full_redraw || line != prev_line {
        execute!(
          out,
          MoveTo(0, i as u16),
          Clear(ClearType::CurrentLine),
          Print(line)
        )
        .unwrap();
      }
    } else {
      // clear remaining lines in top region
      execute!(out, MoveTo(0, i as u16), Clear(ClearType::CurrentLine)).unwrap();
    }
  }

  // cursor stays on the last line of the rendered region
  if draw_height > 0 {
    execute!(out, MoveTo(0, (draw_height - 1) as u16)).unwrap();
  }
  out.flush().unwrap();
}

fn print_bottom_bar<W: Write>(out: &mut W, status: &str) -> std::io::Result<()> {
  let (_, terminal_height) = terminal::size()?;
  let last_y = terminal_height.saturating_sub(1);
  // move to the last line and overwrite the existing content
  let (_, width) = terminal::size()?;
  let vis = get_visible_len_for(status);
  let trailing_len = if vis < width as usize {
    width as usize - vis
  } else {
    0
  };
  let trailing = " ".repeat(trailing_len);
  execute!(
    out,
    MoveTo(0, last_y),
    ResetColor,
    Print(status),
    Print(trailing)
  )?;
  out.flush()?;
  Ok(())
}

fn get_visible_len_for(s: &str) -> usize {
  let mut len = 0usize;
  let mut chars = s.chars();
  while let Some(c) = chars.next() {
    if c == '\x1b' {
      // skip ANSI sequences
      while let Some(next) = chars.next() {
        if next == 'm' {
          break;
        }
      }
    } else {
      let double = matches!(c, '🤔' | '🎤' | '🔊');
      len += if double { 2 } else { 1 };
    }
  }
  len
}

fn update_bottom_bar(
  ui: &crate::state::UiState,
  status_line: &Arc<Mutex<String>>,
  peak: &Arc<Mutex<f32>>,
  spinner: &[&str],
  i: &mut usize,
) -> String {
  let state = GLOBAL_STATE.get().expect("AppState not initialized");
  let speak = state.ui.agent_speaking.load(Ordering::Relaxed);
  let think = ui.thinking.load(Ordering::Relaxed);
  let play = state.ui.playing.load(Ordering::Relaxed);
  let recording_paused = state.recording_paused.load(Ordering::Relaxed);
  let conversation_paused = state.conversation_paused.load(Ordering::Relaxed);
  let status = if recording_paused {
    "⏸️".to_string()
  } else if play {
    format!("🔊 {}", spinner[*i % spinner.len()])
  } else if speak {
    format!("🎤 {}", spinner[*i % spinner.len()])
  } else if think {
    format!("🤔 {}", spinner[*i % spinner.len()])
  } else {
    format!("🎤 {}", spinner[*i % spinner.len()])
  };
  let (cols_raw, _x) = terminal::size().unwrap_or((80, 24));
  let cols = cols_raw as usize;
  let peak_val = match peak.lock() {
    Ok(v) => *v,
    Err(_) => 0.0,
  };
  let speed_str = format!("[{:.1}x]", get_speed());
  let voice_str = format!("({})", get_voice());
  let recording_paused_str = if recording_paused {
    "\x1b[43m\x1b[30m  paused  \x1b[0m"
  } else {
    "\x1b[41m\x1b[37m listening \x1b[0m"
  };
  let recording_paused_vis_len = get_visible_len_for(recording_paused_str);
  let internal_status = format!(
    "{}{}{}{}",
    if recording_paused {
      "\x1b[47m█\x1b[0m"
    } else {
      "\x1b[100m█\x1b[0m"
    },
    if conversation_paused {
      "\x1b[47m█\x1b[0m"
    } else {
      "\x1b[100m█\x1b[0m"
    },
    if state.playback.paused.load(Ordering::Relaxed) {
      "\x1b[100m█\x1b[0m"
    } else {
      "\x1b[47m█\x1b[0m"
    },
    if state.playback.playback_active.load(Ordering::Relaxed) {
      "\x1b[47m█\x1b[0m"
    } else {
      "\x1b[100m█\x1b[0m"
    }
  );
  let combined_status = format!("{} {} ", voice_str, internal_status);
  let available = cols.saturating_sub(
    get_visible_len_for(&status)
      + 2
      + get_visible_len_for(&combined_status)
      + 1
      + get_visible_len_for(&speed_str)
      + recording_paused_vis_len,
  );
  let max_bar_len = if available > 40 { 40 } else { available };
  let mut bar_len = ((peak_val * (max_bar_len as f32)).round() as usize).min(max_bar_len);
  if recording_paused {
    bar_len = 0;
  }
  let bar_color = if recording_paused {
    "\x1b[37m"
  } else if speak {
    "\x1b[31m"
  } else {
    "\x1b[37m"
  };
  let bar = format!("{}{}\x1b[0m", bar_color, "█".repeat(bar_len));
  let spaces = cols.saturating_sub(
    get_visible_len_for(&status)
      + 2
      + bar_len
      + get_visible_len_for(&speed_str)
      + get_visible_len_for(&combined_status)
      + recording_paused_vis_len,
  );
  let status_without_speed = format!("{} {}{}", status, bar, " ".repeat(spaces));
  let full_bar = format!(
    "{}{} {}{}",
    status_without_speed, speed_str, combined_status, recording_paused_str
  );
  if let Ok(mut st) = status_line.lock() {
    *st = full_bar.clone();
  }
  full_bar
}
