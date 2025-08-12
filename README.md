# Patchdog

Creates a draft PR that after approved infers into your main PR with filled in comments, if you choose to approve it will appear as part of your base branch, hence, your main PR stays open.

You can see it in action here: https://github.com/YuraLitvinov/patchdog/pull/19 commit: b6c8a972d768d6da1a280b7b060629597c0cc160

Requires .env file to function on local system or providing secrets to Github Actions. Insert your key and from there everything should work. The script is bound to Rust files, as it uses parsing to avoid broken code.  The project aims to be ran as an Actions. Essentially, this tool manipulates a patch file, finds all changes and creates documentation for them.
Plans:
1. Differentiate between trait functions and normal functions.
2. Add context to request, so LLM can produce better documentation.
3. Cleanup existing comments, replacing them with new ones - if this will prove reasonable

Build prerequisites: If you choose glibc version for running on your host, you can use cargo build to setup things for you but you must provide all necessary dependecies: 
openssl, pkg-config 
Else, set-up docker for statically-linked musl version, run build.sh

Interface description: your entry point into the program is patch parsing which occurs in patchdog/src/binding.rs, git_parsing/src/patch_parse.rs and Rust object parsing under rust_parsing crate - it provides an inteface via RustItemParser and RustParser trait, you may call any function available there, also, rust_parsing provides additional filesystem manipulation, you may observe it as Files and FileExtractor traits. Those are a list of public functions that are used within the project.
The information that was processed in previous steps then gets exported as SingleFunctionData object - it's grouped as it's necessary for receiving proper input. Metadata field that is excluded from serialization, hence, LLM doesn't this redundant info, and not wasting tokens and increasing possibility for hallucinations. 
All this grouped SingleFunction data information is collected into batches of MappedRequest - it has a limit of tokens set by tokens_per_min. It's 250_000 for the chosen model, as set in config.yaml and then grouped again, into greater collection of WaitForTimeout which is also a defined limit of request_per_min. It's 10, as those are the rate limit for gemini API. You can get this information from google's ai studio website, specify the chosen model and configure your personal limits.
After we have acquired the answer,, we come to an important step, where all this returned data has to be tried for proper return of UUIDs. Usually, one or two answers are broken and call() function automatically manages this if pool_of_request doesn't end up empty - it gets called recursively, hence, repeating all the process again.

# Please, do not hesitate to contact me if you run into certain issues while using the provided service, although nothing is guaranteed: litvinov.yura@gmail.com