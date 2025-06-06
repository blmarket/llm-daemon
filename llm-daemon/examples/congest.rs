use llm_daemon::Daemon2;
use llm_daemon::LlmDaemon;

fn main() {
    let t1 = Daemon2::from("ggml-org/Qwen2.5-Coder-7B-Q8_0-GGUF".to_string());
    
    dbg!(t1.config());
}