pub mod parse_json;

pub fn return_prompt() -> &'static str {
    r#"The provided data is a collection valid Rust code.
      
    "uuid": {
        "fn_name": "fn_name",
        "function_text": "function_text" 
    }, 
    Instruction: Generate a comment for each in 'function_text', write it into 'new_comment' 
    field for each 'function_text' in the structure, match uuid and fn_name.
    if 'context' field exists, use it at your disposal, elsewise proceed as is, generate
    /// comment for it and fill in the new_comment block. Return type should be a JSON object of this type:
    [
    	{
        	"uuid": "",
        	"fn_name": "",
        	"new_comment": ""
    	}
    ]
    Each new object should be located inside [] block. Do not wrap it as markdown block. response_format = {"type": "json_object"}"#
}
