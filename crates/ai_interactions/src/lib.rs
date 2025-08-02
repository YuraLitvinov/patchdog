pub mod parse_json;

pub fn return_prompt() -> &'static str {
    r#"response_format = {"type": "json_object"} The provided data is a collection of valid Rust code.
    [
        {
            "uuid": "uuid", 
            "data": {
                "fn_name": "fn_name",
                "function_text": "function_text" 
            }
        } 
    ]
    Instruction: Generate a comment containing return type, parameters and description for each of 'data' 'function_text' corresponding to it's 'uuid' and 'fn_name', write it into 'new_comment' 
    field and match uuid and fn_name. If 'context' field exists, use it at your disposal, elsewise proceed as is, generate /// comment for it and fill in the new_comment block. Each new object should be located inside [] block. Return type should be a JSON object of this type:
    [
    	{
        	"uuid": "",
        	"fn_name": "",
        	"new_comment": ""
    	}
    ]"#
}
