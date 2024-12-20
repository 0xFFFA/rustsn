use crate::cache::Cache;
use crate::llm_prompt::Prompt;
use crate::{OLLAMA_API, OLLAMA_EMB, VERBOSE};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// const STOP_WORDS: &[&str] = &[
//     "**Explanation",
//     "**Notes",
//     "### Explanation",
//     "**Additional Notes",
// ];
const STOP_WORDS: &[&str] = &[];
const MAX_TOKENS: i32 = 1000;
pub struct LLMApi {
    model_type: ModelType,
}

#[derive(Debug, PartialEq)]
pub enum ModelType {
    Ollama { model: String, emb: String },
    OpenAI { api_key: String },
}

impl LLMApi {
    pub fn new(model_type: ModelType) -> LLMApi {
        LLMApi { model_type }
    }

    pub fn request(
        &self,
        prompt_template: &str,
        params: &Vec<String>,
        cache: &mut Cache,
        prompt: &Prompt,
    ) -> String {
        let prompt = if params.len() > 0 {
            prompt.create(prompt_template, params)
        } else {
            prompt_template.to_string()
        };
        match &self.model_type {
            ModelType::Ollama { model, .. } => {
                let stop = STOP_WORDS;
                let request = OllamaRequest {
                    // model: "qwen2.5-coder:7b".to_string(), // smart model but slow
                    // model: "qwen2.5-coder:1.5b".to_string(), // smart model but slow
                    model: model.to_string(),
                    // model: "gemma2:2b".to_string(), // fast but very stupid model - excellent for fast testing
                    //  model: "gemma2".to_string(), // medium model
                    prompt: prompt.to_string(),
                    stream: false,
                    options: OllamaOptions {
                        num_predict: MAX_TOKENS,
                        stop: stop.iter().map(|s| s.to_string()).collect(),
                    },
                };

                let request_str = serde_json::to_string(&request).unwrap();
                if *VERBOSE.lock().unwrap() {
                    println!("Request: {}", request.prompt);
                }

                let response_opt = cache.get(&request_str);
                let response = match response_opt {
                    None => {
                        let client = Client::builder()
                            .timeout(Duration::from_secs(60 * 10))
                            .build()
                            .unwrap();
                        println!("Request to LLM in progress");

                        let response = client.post(OLLAMA_API).json(&request).send().unwrap();
                        if !response.status().is_success() {
                            let response_text = response.text().unwrap();
                            println!("Response: {:?}", response_text);
                            panic!("Failed to get response from LLM");
                        }
                        let response_text = response.text().unwrap();
                        let response =
                            serde_json::from_str::<OllamaResponse>(&response_text).unwrap();
                        cache.set(request_str.clone(), response.response.clone());
                        response.response
                    }
                    Some(result) => {
                        println!("LLM Request already cached");
                        result.to_string()
                    }
                };

                if *VERBOSE.lock().unwrap() {
                    println!("Response: {}", response);
                }
                response
            }
            ModelType::OpenAI { api_key } => {
                let messages = vec![ChatMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                }];

                let request = OpenAIChatRequest {
                    model: "gpt-4o-2024-08-06".to_string(),
                    messages,
                    max_tokens: MAX_TOKENS,
                    temperature: 0.7,
                    stop: Some(STOP_WORDS.iter().map(|s| s.to_string()).collect()),
                };

                let request_str = serde_json::to_string(&request).unwrap();
                if *VERBOSE.lock().unwrap() {
                    println!("Request: {}", prompt);
                }

                let response_opt = cache.get(&request_str);
                let response = match response_opt {
                    None => {
                        println!("Request to LLM in progress");
                        let client = Client::builder()
                            .timeout(Duration::from_secs(60 * 5))
                            .build()
                            .unwrap();

                        let response = client
                            .post("https://api.openai.com/v1/chat/completions")
                            .bearer_auth(api_key)
                            .json(&request)
                            .send()
                            .unwrap()
                            .json::<OpenAIChatResponse>()
                            .unwrap();

                        // Extract the assistant's reply from the first choice
                        let openai_response = response
                            .choices
                            .into_iter()
                            .next()
                            .map(|choice| choice.message.content)
                            .unwrap_or_default();

                        cache.set(request_str.clone(), openai_response.clone());
                        openai_response
                    }
                    Some(result) => {
                        println!("LLM Request already cached");
                        result.to_string()
                    }
                };

                if *VERBOSE.lock().unwrap() {
                    println!("OpenAI Chat Response: {}", response);
                }
                response
            }
        }
    }
    pub fn emb(&self, content: &str, cache: &mut Cache, full_content: &str) -> Vec<f32> {
        match &self.model_type {
            ModelType::Ollama { emb, .. } => {
                let request = OllamaEmbRequest {
                    model: emb.to_string(),
                    prompt: full_content.to_string(),
                };

                let request_str = serde_json::to_string(&content).unwrap();
                let response_opt = cache.get(&request_str);
                let response = match response_opt {
                    None => {
                        println!("Request to Ollama Embeddings API in progress");
                        let client = Client::builder()
                            .timeout(Duration::from_secs(60 * 10))
                            .build()
                            .unwrap();
                        // let response = client
                        //     .post(OLLAMA_EMB)
                        //     .json(&request)
                        //     .send()
                        //     .unwrap()
                        //     .json::<OllamaEmbResponse>()
                        //     .unwrap();
                        //
                        let response_str = client
                            .post(OLLAMA_EMB)
                            .json(&request)
                            .send()
                            .unwrap()
                            .text()
                            .unwrap();
                        // println!("Response: {}", response_str);
                        let response: OllamaEmbResponse =
                            serde_json::from_str(&response_str).unwrap();
                        cache.set(
                            request_str.clone(),
                            serde_json::to_string(&response.embedding).unwrap(),
                        );
                        response.embedding
                    }
                    Some(result) => {
                        println!("Embedding Request already cached");
                        serde_json::from_str(&result).unwrap()
                    }
                };
                response
            }
            ModelType::OpenAI { api_key } => {
                let request = OpenAIEmbRequest {
                    model: "text-embedding-ada-002".to_string(),
                    input: full_content.to_string(),
                };

                let request_str = serde_json::to_string(&request).unwrap();

                let response_opt = cache.get(&content);

                let response = match response_opt {
                    None => {
                        let client = Client::builder()
                            .timeout(Duration::from_secs(60 * 5))
                            .build()
                            .unwrap();

                        println!("Request to OpenAI Embeddings API in progress");

                        let api_response = match client
                            .post("https://api.openai.com/v1/embeddings")
                            .bearer_auth(api_key)
                            .json(&request)
                            .send()
                        {
                            Ok(resp) => resp,
                            Err(e) => {
                                eprintln!("Network error: {}", e);
                                return vec![];
                            }
                        };

                        let api_response = match api_response.json::<OpenAIEmbResponse>() {
                            Ok(json) => json,
                            Err(e) => {
                                eprintln!("Failed to parse JSON response: {}", e);
                                return vec![];
                            }
                        };

                        cache.set(
                            request_str.clone(),
                            serde_json::to_string(&api_response.data[0].embedding).unwrap(),
                        );
                        api_response.data[0].embedding.clone()
                    }
                    Some(result) => {
                        println!("Embedding Request already cached");
                        serde_json::from_str(&result).unwrap()
                    }
                };

                if *VERBOSE.lock().unwrap() {
                    println!("OpenAI Embedding Response: {:?}", response);
                }
                response.to_vec()
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaOptions {
    num_predict: i32,
    stop: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaResponse {
    model: String,
    created_at: String,
    response: String,
    done: bool,
    done_reason: String,
    context: Vec<i64>,
    total_duration: i64,
    load_duration: i64,
    prompt_eval_count: i32,
    prompt_eval_duration: i64,
    eval_count: i32,
    eval_duration: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaEmbRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaEmbResponse {
    embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: i32,
    temperature: f32,
    stop: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String, // e.g., "user", "assistant", "system"
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIChatResponse {
    id: String,
    object: String,
    created: i64,
    choices: Vec<OpenAIChatChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIChatChoice {
    index: i32,
    message: ChatMessage, // Changed to include the message object
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIEmbRequest {
    model: String,
    input: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIEmbResponse {
    data: Vec<OpenAIEmbData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIEmbData {
    embedding: Vec<f32>,
}
