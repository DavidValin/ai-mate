// ------------------------------------------------------------------
//  debate
// ------------------------------------------------------------------

use std::sync::atomic::AtomicU64;
use tokio::runtime::Builder as TokioBuilder;
use crate::conversation::ChatMessage;
use crossbeam_channel::{unbounded};
use std::sync::Arc;
use crate::state::GLOBAL_STATE;

async fn debate_get_response(messages: Vec<ChatMessage>, agent: &crate::config::AgentSettings) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
  let (_stop_tx, stop_rx) = unbounded::<()>();
  let interrupt_counter = Arc::new(AtomicU64::new(0));
  let mut result = String::new();
  let mut on_piece = |piece: &str| {
    result.push_str(piece);
  };
  crate::llm::llama_server_stream_response_into(
    &messages,
    &agent.baseurl,
    &agent.model,
    &agent.provider,
    &stop_rx,
    interrupt_counter.clone(),
    0,
    &mut on_piece,
  ).await?;
  Ok(result)
}

pub fn run_debate(subject: String, agents: Vec<crate::config::AgentSettings>, tx_tts: crossbeam_channel::Sender<(String, u64, String)>, tx_ui: crossbeam_channel::Sender<String>, interrupt_counter: std::sync::Arc<std::sync::atomic::AtomicU64>, pending_user: std::sync::Arc<std::sync::Mutex<Option<crate::conversation::ChatMessage>>>, tts_done_rx: crossbeam_channel::Receiver<()>){
  if agents.len() < 2 {
    eprintln!("Not enough agents to debate. At least two required.");
    return;
  }
  let agent_count = agents.len();
  let rt = TokioBuilder::new_current_thread().enable_all().build().unwrap();
  let mut turn = 0usize;
  let mut previous_reply = String::new();
  let mut history: Vec<crate::conversation::ChatMessage> = Vec::new();
  loop {
    // Check for pending user input
    let mut pending_msg_opt: Option<crate::conversation::ChatMessage> = None;
    {
        let mut lock = pending_user.lock().unwrap();
        if let Some(msg) = lock.take() {
            pending_msg_opt = Some(msg);
        }
    }
    let current_agent = if let Some(_) = pending_msg_opt {
        &agents[0]
    } else {
        &agents[turn % agent_count]
    };
    let system_prompt = current_agent.system_prompt.replace("\\n", "\n");
    let user_msg = if let Some(msg) = pending_msg_opt {
        // Reset turn for new user query
        turn = 0;
        previous_reply.clear();
        msg.content
    } else if turn == 0 {
        format!("{}. Respond as short as possible", subject)
    } else {
        previous_reply.clone()
    };
     let mut messages = history.clone();
     messages.push(ChatMessage { role: "system".to_string(), content: system_prompt.clone() });
      messages.push(ChatMessage { role: "user".to_string(), content: user_msg });
     let reply = rt.block_on(debate_get_response(messages, current_agent)).unwrap_or_else(|e| {
        eprintln!("Error getting response: {}", e);
        std::process::exit(1);
      });
      // Append assistant reply to conversation history for subsequent turns
      history.push(ChatMessage{role:"assistant".to_string(), content: reply.clone()});
      let _ = tx_ui.send("line| ".to_string());
     let label = format!("\x1b[48;5;22;37m{}:\x1b[0m", current_agent.name);
     let _ = tx_ui.send(format!("line|{}", label));
     let _ = tx_ui.send(format!("line|{}", reply.trim()));
     let current_interrupt = interrupt_counter.load(std::sync::atomic::Ordering::SeqCst);
     {
       let state = GLOBAL_STATE.get().expect("AppState not initialized");
       let original_voice = { let v = state.voice.lock().unwrap(); v.clone() };
       let original_tts = { let v = state.tts.lock().unwrap(); v.clone() };
       { let mut v = state.voice.lock().unwrap(); *v = current_agent.voice.clone(); }
       { let mut v = state.tts.lock().unwrap(); *v = current_agent.tts.clone(); }
       let phrases = split_into_phrases(&reply);
       for phrase in phrases {
           let cleaned = strip_special_chars(&phrase);
           let _ = tx_tts.send((cleaned, current_interrupt, current_agent.voice.clone()));
           let _ = tts_done_rx.recv();
       }
       { let mut v = state.voice.lock().unwrap(); *v = original_voice; }
       { let mut v = state.tts.lock().unwrap(); *v = original_tts; }
     }
     previous_reply = reply.trim().to_string();
     turn += 1;
  }
}

// Utility to split a large reply into phrase chunks similar to conversation.rs
fn split_into_phrases(text: &str) -> Vec<String> {
    let mut phrases = Vec::new();
    let mut buf = String::new();
    for c in text.chars() {
        buf.push(c);
        if c == '\n' || c == '.' {
            let trimmed = buf.trim();
            if !trimmed.is_empty() {
                phrases.push(trimmed.to_string());
            }
            buf.clear();
        }
    }
    if !buf.trim().is_empty() {
        phrases.push(buf.trim().to_string());
    }
    phrases
}

// Utility to strip special chars as conversation.rs does before TTS
fn strip_special_chars(s: &str) -> String {
    let mut result = String::new();
    let parts: Vec<&str> = s.split("```").collect();
    let mut inside = false;
    for (i, part) in parts.iter().enumerate() {
        if !inside {
            result.extend(part.chars().filter(|c| {
                ![
                    '+', '.', '~', '*', '&', '-', ',', ';', ':', '(', ')', '[', ']', '{', '}', '"', '\'',
                    '#', '`', '|',
                ].contains(c)
            }));
        }
        if i < parts.len() - 1 {
            inside = !inside;
        }
    }
    result
}
