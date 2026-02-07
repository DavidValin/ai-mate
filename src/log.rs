use std::io::Write;

pub fn log(msg_type: &str, msg: &str) {
    let mut out = std::io::stdout();
    let emoji = match msg_type {
        "debug" => "🐛",
        "info" => "ℹ️",
        "warning" => "⚠️",
        "error" => "❌",
        _ => "",
    };
    write!(out, "\r\x1b[K{}  {}\n", emoji, msg).unwrap();
    out.flush().unwrap();
}
