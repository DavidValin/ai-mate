// ------------------------------------------------------------------
//  debate
// ------------------------------------------------------------------

use std::sync::atomic::AtomicU64;
use tokio::runtime::Builder as TokioBuilder;
use crate::conversation::ChatMessage;
use crossbeam_channel::{unbounded};
use std::sync::{Arc};
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

pub fn run_debate(subject: String, agents: Vec<crate::config::AgentSettings>, tx_tts: crossbeam_channel::Sender<(String, u64, String)>, tx_ui: crossbeam_channel::Sender<String>, interrupt_counter: std::sync::Arc<std::sync::atomic::AtomicU64>, tts_done_rx: crossbeam_channel::Receiver<()>) {
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
    let current_agent = &agents[turn % agent_count];
    let system_prompt = current_agent.system_prompt.replace("\\n", "\n");
     let user_msg = if turn == 0 {
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
    let LABEL = format!("\x1b[48;5;22;37m{}:\x1b[0m", current_agent.name);
    let _ = tx_ui.send(format!("line|{}", LABEL));
    let _ = tx_ui.send(format!("line|{}", reply.trim()));
    let current_interrupt = interrupt_counter.load(std::sync::atomic::Ordering::SeqCst);
    {
      let state = GLOBAL_STATE.get().expect("AppState not initialized");
      let original_voice = { let v = state.voice.lock().unwrap(); v.clone() };
      let original_tts = { let v = state.tts.lock().unwrap(); v.clone() };
      { let mut v = state.voice.lock().unwrap(); *v = current_agent.voice.clone(); }
      { let mut v = state.tts.lock().unwrap(); *v = current_agent.tts.clone(); }
      let res = tx_tts.send((reply.clone(), current_interrupt, current_agent.voice.clone()));
      let _ = res;
      // Wait for TTS to finish before next turn
      let _ = tts_done_rx.recv();
      { let mut v = state.voice.lock().unwrap(); *v = original_voice; }
      { let mut v = state.tts.lock().unwrap(); *v = original_tts; }
    }
    previous_reply = reply.trim().to_string();
    turn += 1;
  }
}