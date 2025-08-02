use ai_interactions::return_prompt;
use async_trait::async_trait;
use gemini_rust::Gemini;
use regex::Regex;
use rust_parsing::error::{ErrorBinding, SerdeSnafu};
use rust_parsing::{ErrorHandling, error::GeminiRustSnafu, error::StdVarSnafu};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use snafu::ResultExt;
use std::ops::Range;
use std::collections::HashMap;
use rust_parsing::file_parsing::REGEX;
use std::{fmt::Display, time, env::var};
//Theoretical maximum is 250_000, but is highly flawed in a way, that Gemini can 'tear' the response. 
//This behavior is explained in call_json_to_rust error case
//Similar issue on https://github.com/googleapis/python-genai/issues/922
const TOKENS_PER_MIN: usize = 250_000;
pub const REQUESTS_PER_MIN: usize = 5;
const TOKENS_PER_REQUEST: usize = TOKENS_PER_MIN / REQUESTS_PER_MIN;
/*

[
    {
        "id": "f81d4fae-7dec-11d0-a765-00a0c91e6bf6",
        "fn-name": "main",
        "comment": "bla-bla",
    },
    {
        "id": "f81d4fae-7dec-11d0-a765-00a0c91e6bf6",
        "fn-name": "new",
        "comment": "bla-bla",
    }
]

*/
#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub uuid: String,
    pub fn_name: String,
    pub new_comment: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct SingleFunctionData {
    pub fn_name: String,
    pub function_text: String,
    #[serde(skip_serializing)]
    pub context: ContextData,
}
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ContextData {
    pub class_name: String,
    pub filepath: String,
    pub external_dependecies: Vec<String>,
    pub old_comment: Vec<String>,
    pub line_range: Range<usize>,
}
impl ContextData {
    pub fn size(&self) -> usize {
        let mut size_ext = 0;
        for each in &self.external_dependecies {
            size_ext += each.len();
        }
        for each in &self.old_comment {
            size_ext += each.len();
        }
        self.class_name.len() + 
        self.filepath.len() + 
        size_ext + 
        self.line_range.len()
    }
}

impl SingleFunctionData {
    pub fn size(&self) -> usize {
        ( self.fn_name.len() + self.context.size() + self.function_text.len()) / 3 //One token is approx. 3 symbols
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MappedRequest {
    pub remaining_capacity: usize,
    pub data: HashMap<String, SingleFunctionData>,
}

impl MappedRequest {
    pub fn new() -> MappedRequest {
        MappedRequest {
            remaining_capacity: TOKENS_PER_REQUEST,
            data: HashMap::new(),
        }
    }
    pub fn function_add(&mut self, request_data: SingleFunctionData) -> bool {
        let size = request_data.size();
        if size <= self.remaining_capacity {
            self.data.insert(uuid::Uuid::new_v4().to_string(), request_data);
            self.remaining_capacity -= size;
            true
        } else {
            false
        }
    }
}

impl Default for MappedRequest {
    fn default() -> Self {
        Self::new()
    }
}
    
impl Display for MappedRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{self:#?}")
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct PreparingRequests {
    pub remaining_capacity: usize,
    pub data: Vec<SingleFunctionData>,
}

impl PreparingRequests {
    pub fn new() -> PreparingRequests {
        PreparingRequests {
            remaining_capacity: TOKENS_PER_REQUEST - return_prompt().len(),
            data: vec![],
        }
    }
    pub fn function_add(&mut self, request_data: SingleFunctionData) -> bool {
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

impl Default for PreparingRequests {
    fn default() -> Self {
        Self::new()
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
    pub prepared_requests: Vec<MappedRequest>,
}
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Request {
    uuid: String,
    data: SingleFunctionData
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
    pub async fn send_batches(request: &Vec<WaitForTimeout>) -> Result<Vec<String>, ErrorHandling> {
        let mut response = vec![];
        let one_minute = time::Duration::from_secs(61);
        for single_request in request {
            for each in &single_request.prepared_requests {
                let fmt = &each.data;
                let mut vec = vec![];
                for (val, each) in fmt {
                    vec.push(Request {
                        uuid: val.clone(),
                        data: each.clone(),
                    });
                }
                let as_json = serde_json::to_string_pretty(&vec).context(SerdeSnafu)?; 
                match GoogleGemini::req_res(&as_json, return_prompt()).await {
                    //Handling exclusive case, where one of the requests may fail
                    Ok(r) => {
                        response.push(r);
                    },
                    Err(e) => {
                        //error marker
                        return Err(e); 
                    }
                }
            }

            if request.len() > 1 {
                tokio::time::sleep(one_minute).await;
            }
        }
        println!("{}", "exited send_batches");
        Ok(response)
    }

    pub fn assess_batch_readiness(
        batch: Vec<MappedRequest>,
    ) -> Result<Vec<WaitForTimeout>, ErrorBinding> {
        //Architecture: batch[BIG_NUMBER..len()-1]
        //Next: batch[0..4]
        let mut await_response: Vec<WaitForTimeout> = vec![];
        if batch.len() > REQUESTS_PER_MIN {
            let mut size: usize = batch.len();
            for _ in 1..=batch.len().div_ceil(REQUESTS_PER_MIN) {
                let mut new_batch: Vec<MappedRequest> = Vec::new();
                //Response where quantity of batches exceed allow per min request count
                //Check for last items in batch
                if size < REQUESTS_PER_MIN {
                    new_batch.extend_from_slice(&batch[0..size]);
                    await_response.push(WaitForTimeout {
                        prepared_requests: new_batch,
                    });
                    continue;
                } else {
                    new_batch.extend_from_slice(
                        &batch[size.saturating_sub(REQUESTS_PER_MIN)..size],
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

    pub async fn req_res(file_content: &str, arguments: &str) -> Result<String, ErrorHandling> {
        dotenv::from_path(".env").unwrap();
        let api_key = var("API_KEY_GEMINI").context(StdVarSnafu)?;
        let model = var("GEMINI_MODEL").context(StdVarSnafu)?;
        let client = Gemini::with_model(
            api_key, 
            model,
        )
            .generate_content()
            .with_system_prompt(arguments)
            .with_user_message(file_content)
            .execute()
            .await
            .context(GeminiRustSnafu)?;

        Ok(client.text())
    }

    // The idea as I see it is: we provide AI Agent with filled out JSON where all the function names are already mapped and
    // the only goal there is to actually to turn in the JSON and receive it back with written in comments
    pub fn prepare_batches(
        &mut self,
        request: Vec<SingleFunctionData>,
    ) -> Result<Vec<PreparingRequests>, ErrorHandling> {
        let mut batches: Vec<PreparingRequests> = Vec::new();
        let mut preparing_requests = PreparingRequests::new();
        for data in request {
            if !preparing_requests.function_add(data.clone()) {
                //Preserving overflow of preparing request to next iter
                if !preparing_requests.data.is_empty() {
                    batches.push(preparing_requests);
                }
                //Reinitializing preparing_requests to free the buffer
                preparing_requests = PreparingRequests::new();

                // Attempt to push
                if !preparing_requests.function_add(data) {
                    // Here should be handled the case, where single object exceeds token limit
                    //Which is likely would not be possible
                }
            }
        }

        // Last unempty request
        if !preparing_requests.data.is_empty() {
            batches.push(preparing_requests);
        }
        Ok(batches)
    }
    
    pub fn prepare_map(
        &mut self,
        request: Vec<SingleFunctionData>,
    ) -> Result<Vec<MappedRequest>, ErrorHandling> {
        let mut batches: Vec<MappedRequest> = Vec::new();
        let mut mapped_requests = MappedRequest::new();
        for data in request {
            if !mapped_requests.function_add(data.clone()) {
                //Preserving overflow of preparing request to next iter
                if !mapped_requests.data.is_empty() {
                    batches.push(mapped_requests);
                }
                //Reinitializing preparing_requests to free the buffer
                mapped_requests = MappedRequest::new();

                // Attempt to push
                if !mapped_requests.function_add(data) {
                    // Here should be handled the case, where single object exceeds token limit
                    //Which is likely would not be possible
                }
            }
        }
        // Last unempty request
        if !mapped_requests.data.is_empty() {
            batches.push(mapped_requests);
        }
        Ok(batches)
    }
}

//Hotfix returns missing elements of request that were dropped in the response by the LLM
pub fn hotfix(response: String, request: Vec<SingleFunctionData>)-> Result<Vec<SingleFunctionData>, ErrorHandling> {
    let mut hotfixed = vec![];
    let as_req: Vec<SingleFunctionData> = collect_response(&response)?;
    let mut map_request  = HashMap::new();
    request.clone().into_iter().for_each(|each| {
        map_request.insert((each.context.filepath.clone(), each.context.line_range.clone()), each);
    });
    let mut map_response = HashMap::new();
    as_req.clone().into_iter().for_each(|each| {
        map_response.insert((each.context.filepath.clone(), each.context.line_range.clone()), each);
    });
    //Key here represents filepath and lineranges of an object, i.e. function
    for (key, data) in map_request {
        if !map_response.contains_key(&key) {
            hotfixed.push(data);
        }
    }
    Ok(hotfixed)
}

//Accepts JSON as Vec<String> and attempts to parse it into PreparingRequests
pub fn collect_response(output: &str) -> Result<Vec<SingleFunctionData>, ErrorHandling> {
    let re = Regex::new(REGEX).unwrap();        
    let mut assess_size = vec![];
    for cap in re.captures_iter(&output) {
        let a = cap
            .get(0)
            .unwrap()
            .as_str();
        let to_struct = serde_json::from_str::<SingleFunctionData>(a)
            .context(SerdeSnafu)?;
        assess_size.push(to_struct);
    }
    Ok(assess_size)


}
