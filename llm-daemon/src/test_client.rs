use std::sync::Arc;
use std::time::Duration;

use futures::{Future, FutureExt as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, info, warn};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Choice {
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Choices {
    choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Completion {
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum LlmResponse {
    OpenAI(Choices),
    LlamaCpp(Completion),
}

pub struct Generator {
    endpoint: Url,
    model: String,
    client: Arc<Client>,
}

impl Generator {
    pub fn new(endpoint: Url, model: Option<String>) -> Self {
        Generator {
            endpoint,
            model: model.unwrap_or_else(|| {
                "HF://mlc-ai/gemma-2b-it-q4f16_1-MLC".to_string()
            }),
            client: Arc::new(Client::new()),
        }
    }

    async fn try_gen(
        endpoint: Url,
        client: Arc<Client>,
        model: String,
        prompt: String,
        max_tokens: Option<usize>,
        stop: Option<Vec<String>>,
    ) -> Result<String, bool> {
        let mut stop = stop.unwrap_or_default();
        stop.push("<eos>".to_string());
        let payload = json!({
            "max_tokens": max_tokens.unwrap_or(128),
            "n_predict": max_tokens.unwrap_or(128),
            "model": model,
            "prompt": prompt,
            "stream": false,
            "stop": stop,
        });

        let res = client.post(endpoint).json(&payload).send().await.map_err(
            |err| {
                if err.is_connect() {
                    debug!("got connect error, better retry");
                    return true;
                }
                false
            },
        )?;

        if res.status() == 503 {
            debug!("got 503 error, better retry");
            return Err(true);
        }
        if !res.status().is_success() {
            return Err(false);
        }

        // Assume all error happening here is unrecoverable
        res.text().await.map_err(|_| false)
    }

    async fn retry<R: Future<Output = Result<String, bool>>, F: Fn() -> R>(
        func: F,
    ) -> anyhow::Result<String> {
        let mut cnt = 30;
        loop {
            let r1 = func().await;
            match r1 {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    if err {
                        info!("Retryable error, retry in 1000ms");
                        let _ = tokio::time::sleep(Duration::from_millis(1000))
                            .await;
                    } else {
                        warn!(
                            err = format!("{:?}", err),
                            "unknown error querying daemon"
                        );
                        anyhow::bail!(err)
                    }
                },
            };
            cnt -= 1;
            if cnt == 0 {
                anyhow::bail!("timed out");
            }
        }
    }

    pub fn generate(
        &self,
        prompt: String,
    ) -> impl Future<Output = anyhow::Result<String>> + Send + 'static {
        let endpoint = self.endpoint.clone();
        let client = self.client.clone();
        let model = self.model.clone();
        Self::retry(move || {
            Self::try_gen(
                endpoint.clone(),
                client.clone(),
                model.clone(),
                prompt.clone(),
                None,
                None,
            )
        })
        .map(|json| match serde_json::from_str(&json?)? {
            LlmResponse::OpenAI(choices) => {
                Ok(choices.choices.first().unwrap().text.clone())
            },
            LlmResponse::LlamaCpp(completion) => Ok(completion.content),
        })
    }
}
