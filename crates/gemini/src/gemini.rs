use ai_interactions::return_prompt;
use async_trait::async_trait;
use gemini_rust::Gemini;
use rust_parsing::error::ErrorBinding;
use rust_parsing::{ErrorHandling, error::GeminiRustSnafu};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::json;
use snafu::ResultExt;
use std::{fmt::Display, thread, time};

const TOKENS_PER_MIN: usize = 250_000;
pub const REQUESTS_PER_MIN: usize = 5;
const TOKENS_PER_REQUEST: usize = TOKENS_PER_MIN / REQUESTS_PER_MIN;
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SingleRequestData {
    pub function_text: String,
    pub context: String,
    pub comment: String,
    pub filepath: String,
}
impl SingleRequestData {
    pub fn size(&self) -> usize {
        (&self.context.len() + &self.function_text.len() + &self.filepath.len()) / 3 //One token is approx. 3 symbols
    }
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PreparingRequests {
    pub remaining_capacity: usize,
    pub data: Vec<SingleRequestData>,
}

impl PreparingRequests {
    pub fn new() -> PreparingRequests {
        PreparingRequests {
            remaining_capacity: TOKENS_PER_REQUEST - return_prompt().len(),
            data: vec![],
        }
    }
    pub fn function_add(&mut self, request_data: SingleRequestData) -> bool {
        let size = request_data.size();
        if size <= self.remaining_capacity {
            self.remaining_capacity -= size;
            self.data.push(request_data);
            true
        } else {
            false
        }
    }
}
impl Display for PreparingRequests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{self:#?}")
    }
}

#[async_trait]
pub trait ReqRes {
    async fn req_res(file_content: String) -> Result<String, ErrorHandling>;
}

#[derive(Debug)]
pub struct GoogleGemini {
    pub preparing_requests: PreparingRequests,
} //Req Res = Request Response

impl Default for GoogleGemini {
    fn default() -> Self {
        Self::new()
    }
}
pub fn json_to<T: DeserializeOwned>(val: serde_json::Value) -> T {
    serde_json::from_value(val).unwrap()
}

#[allow(dead_code)]
pub struct WaitForTimeout {
    prepared_requests: Vec<PreparingRequests>,
}
#[allow(async_fn_in_trait)]
impl GoogleGemini {
    pub fn new() -> GoogleGemini {
        GoogleGemini {
            preparing_requests: PreparingRequests {
                remaining_capacity: TOKENS_PER_MIN / REQUESTS_PER_MIN,
                data: vec![],
            },
        }
    }
    pub async fn send_batches(request: Vec<WaitForTimeout>) -> Result<(), ErrorHandling> {
        if request.len() > 1 {
            let one_minute = time::Duration::from_secs(61);

            for single_request in request {
                for each in single_request.prepared_requests {
                    let as_json = format!("{:?}", json!(each));
                    GoogleGemini::req_res(as_json, return_prompt()).await?;
                }
                thread::sleep(one_minute);
            }
        } else{
            for single_request in request {
                for each in single_request.prepared_requests {
                    let as_json = format!("{:?}", json!(each));
                    GoogleGemini::req_res(as_json, return_prompt()).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn assess_batch_readiness(
        batch: Vec<PreparingRequests>,
    ) -> Result<Vec<WaitForTimeout>, ErrorBinding> {
        //Architecture: batch[BIG_NUMBER..len()-1]
        //Next: batch[0..4]
        let mut await_response: Vec<WaitForTimeout> = vec![];
        if batch.len() > REQUESTS_PER_MIN {
            let mut size: usize = 0;
            for _ in &batch {
                size += 1;
            }
            for _ in 1..=batch.len().div_ceil(REQUESTS_PER_MIN) {
                let mut new_batch: Vec<PreparingRequests> = Vec::new();
                //Response where quantity of batches exceed allow per min request count
                println!("SIZE BEFORE: {}", size);
                //Check for last items in batch
                if size < REQUESTS_PER_MIN {
                    new_batch.extend_from_slice(
                        &batch[size.checked_sub(REQUESTS_PER_MIN).unwrap_or(0)..size - 1],
                    );
                    await_response.push(WaitForTimeout {
                        prepared_requests: new_batch,
                    });
                    continue;
                } else {
                    new_batch.extend_from_slice(
                        &batch[size.checked_sub(REQUESTS_PER_MIN).unwrap_or(0)..size],
                    );
                    size -= REQUESTS_PER_MIN;
                    await_response.push(WaitForTimeout {
                        prepared_requests: new_batch,
                    });
                }
            }
        } else {
            //Return as normal
            await_response.push(WaitForTimeout {
                prepared_requests: batch,
            });
        }

        Ok(await_response)
    }

    pub async fn req_res(file_content: String, arguments: String) -> Result<String, ErrorHandling> {
        //let api_key = std::env::var("API_KEY_GEMINI").context(StdVarSnafu)?;
        let client = Gemini::new("AIzaSyCqlP-v467ts_yN8POCh1ojijXjd0uRwqc");
        //let args = std::env::var("INPUT_FOR_MODEL")?;
        let res = client
            .generate_content()
            .with_system_prompt(arguments)
            .with_user_message(file_content)
            .execute()
            .await
            .context(GeminiRustSnafu)?;
        Ok(res.text())
    }

    // The idea as I see it is: we provide AI Agent with filled out JSON where all the function names are already mapped and
    // the only goal there is to actually to turn in the JSON and receive it back with written in comments
    pub fn prepare_batches(&mut self, request: Vec<SingleRequestData>) -> Vec<PreparingRequests> {
        let mut batches: Vec<PreparingRequests> = Vec::new();
        let mut preparing_requests = PreparingRequests::new();

        for data in request {
            if !preparing_requests.function_add(data.to_owned()) {
                //Preserving overflow of preparing request to next iter
                if !preparing_requests.data.is_empty() {
                    batches.push(preparing_requests);
                }
                //Reinitializing preparing_requests to free the buffer
                preparing_requests = PreparingRequests::new();

                // Attempt to push
                if !preparing_requests.function_add(data) {
                    // Here should be handled the case, where single object exceeds token limit
                }
            }
        }

        // Last unempty request
        if !preparing_requests.data.is_empty() {
            batches.push(preparing_requests);
        }
        batches
    }
}
