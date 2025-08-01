use langchain_rust::language_models::llm::LLM;
use langchain_rust::language_models::options::CallOptions;
use langchain_rust::llm::{OpenAI, OpenAIConfig};
use langchain_rust::schemas::Message;
use llm_daemon::{Daemon3 as Daemon, LlmConfig, LlmDaemon};

/// Even though this example uses langchain_rust, I don't support it for usages.
/// Seems the library is quite big so I stepped back from using it.
fn main() -> anyhow::Result<()> {
    let daemon = Daemon::new(
        "unsloth/Qwen3-Coder-30B-A3B-Instruct-GGUF:Q4_K_XL".to_string(),
        vec![
            "--jinja".to_string(),
            "-ngl".to_string(),
            "99".to_string(),
            "--threads".to_string(),
            "-1".to_string(),
            "--ctx-size".to_string(),
            "32684".to_string(),
            "--temp".to_string(),
            "0.7".to_string(),
            "--min-p".to_string(),
            "0.0".to_string(),
            "--top-p".to_string(),
            "0.80".to_string(),
            "--top-k".to_string(),
            "20".to_string(),
            "--repeat-penalty".to_string(),
            "1.05".to_string(),
        ],
    );
    daemon.fork_daemon()?;
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.spawn(daemon.heartbeat());
    runtime.block_on(async {
        daemon.ready().await;
        // FIXME: Use endpoint provided by daemon
        // daemon needs startup time
        let oai: OpenAI<OpenAIConfig> = OpenAI::new(
            OpenAIConfig::new()
                .with_api_base(daemon.config().endpoint().to_string()),
        );
        let msg0 = Message::new_human_message(
            "Which model are you? Which company created the model?",
        );
        let resp1 = oai.generate(&[msg0]).await?;
        dbg!(resp1);

        let msg1 = Message::new_human_message("What is the sum of 7 and 8?");
        let msg2 = Message::new_ai_message("The sum of 7 and 8 is ");
        let mut oai2 = oai.clone();
        oai2.add_options(CallOptions {
            max_tokens: Some(20),
            stop_words: Some(vec![".".to_string()]),
            ..Default::default()
        });
        let resp2 = oai2.generate(&[msg1, msg2]).await?;
        dbg!(&resp2.generation);
        assert!(resp2.generation.to_string().contains("15"));

        Ok(())
    })
}
