use langchain_rust::language_models::llm::LLM;
use langchain_rust::language_models::options::CallOptions;
use langchain_rust::llm::{OpenAI, OpenAIConfig};
use langchain_rust::schemas::Message;
use llm_daemon::{LlamaConfig, LlamaDaemon, LlmConfig, LlmDaemon};

/// Even though this example uses langchain_rust, I don't support it for usages.
/// Seems the library is quite big so I stepped back from using it.
fn main() -> anyhow::Result<()> {
    let daemon = LlamaDaemon::new(LlamaConfig::default());
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
        let msg0 = Message::new_human_message("Hello, how are you?");
        let resp1 = oai.generate(&[msg0]).await?;
        dbg!(resp1);

        let msg1 = Message::new_human_message("What is the sum of 7 and 8?");
        let msg2 = Message::new_ai_message("The sum of 7 and 8 is ");
        let mut oai2 = oai.clone();
        oai2.add_options(CallOptions {
            max_tokens: Some(1),
            stop_words: Some(vec![".".to_string()]),
            ..Default::default()
        });
        let resp2 = oai2.generate(&[msg1, msg2]).await?;
        assert_eq!(resp2.generation, "15");

        Ok(())
    })
}
