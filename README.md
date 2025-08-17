# Patchdog

**Patchdog** is a development tool that creates a **draft pull request** containing generated documentation for your code changes.  
When the draft PR is approved, it merges into your **main PR** with all included commits.  
The goal is to save time, make code clearer by providing concise and well thought-through comments for your functions.

**Example:**  
[Pull Request #19](https://github.com/YuraLitvinov/patchdog/pull/19) — commit `b6c8a972d768d6da1a280b7b060629597c0cc160`

---

## Features

- Generates documentation comments for changed code automatically.
- Works specifically with **Rust** source files: 
	- tries to establish context links
	- avoids files, that wouldn't compile
- Identifies changed functions and gathers relevant context.
- Respects API rate limits and token budgets for the configured LLM.
- Runs locally or in GitHub Actions.
- Automatically detects if the are conflicts present with base branch, halting further execution
- Gracefully merges into your main PR without any conflicts after patchdog PR into your head branch is submitted
- The Action itself is free of charge. API rates are determined by Google, or other provider of such service when the support will be provided in future for local models, Anthropic and OpenAI.
---

## Requirements


### GitHub Actions

- Check out the repository, provide:

	- Github token

	- API key for Gemini

	- [Configuration file](config.yaml) where your personal settings are stored. You may manipulate the prompt as well to get more verbose or compact comments. Config has to be located inside your root directory. 
```yaml
Patchdog:
    prompt: | 
        response_format = {"type": "json_object"} The provided data is a collection of valid Rust code.
        [
            {
                "uuid": "", 
                "data": {
                    "fn_name": "",
                    "function_text": "" 
                    "context": {
                        "class_name": "",
                        "external_dependencies": [],
                        "old_comment": []
                    }
                }
            } 
        ]
        Instruction: Clone the request form, remove 'data' and append new field 'new_comment', generate, making strong assumptions about code functionality, 
        Generate rustdoc /// comment specify return, input and functionality of the function, with emphasis on functionality - 2-3 sentences per 'data'. 
        If present, use 'external_dependencies' as help, if you run into some sort of misunderstaing. Each new object should be located inside [] block. Return type should be a JSON object of this type:
        [
            {
                "uuid": "", 
                "new_comment": ""
            } 
        ]
    LLM_settings:     
        GEMINI_MODEL: models/gemini-2.5-flash
        TOKENS_PER_MIN: 250000
        REQUESTS_PER_MIN: 10
        OPENAI_MODEL: gpt-3.5-turbo
    Patchdog_settings:
        excluded_files: [tests/, crates/patchdog/src/tests.rs, crates/rust_parsing/src/error.rs]
        excluded_functions: [new, default, main]

```
- You may get the reference for setting up patchdog inside [patchdog repository](.github/workflows/patchdog.yml), or 
```yaml
name: Quickstart

on:
# You may specify the type of PR that triggers the action
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  run_patchdog:
#only linux is supported, but many flavors, as patchdog binaries are statically linked
    runs-on: ubuntu-latest
#these are the permission without which the actions wouldn't work
    permissions: 
      contents: write
      pull-requests: write

    steps:
# important step here, here we clone the repository, so our action is able to access it
      - name: Checkout repository 
        uses: actions/checkout@v4
        with:
          ref: ${{ github.head_ref }}
          fetch-depth: 0
      - name: Run Patchdog action
        uses: ./ # marketplace-defined path
        with: 
#only crucial variables are being passed to patchdog in this case, you may as well change commit author signature
#by default it is Patchdog, some@email.com
          github_token: ${{ secrets.GITHUB_TOKEN }}
          api_key_gemini: ${{ secrets.API_KEY_GEMINI }}
# by default, actions can't get PR metadata by themselves, so we have to pass this metadata ourselves
# provided variable are all the required variables,
# you may as well use aforementioned commit author signature patchdog_name and patchdog_email
          base_branch: ${{ github.event.pull_request.base.ref }}
          head_branch: ${{ github.event.pull_request.head.ref }} 
          assignee: ${{ github.event.pull_request.user.login }} 
```


## Internal composition

### Running locally

- Rust toolchain installed.

- .env file containing your secret API key.

---

### Build Instructions
#### Clone the repository 
```bash
git clone git@github.com:YuraLitvinov/patchdog.git
```
#### Build on your host system for (glibc) --dynamic
```bash
cargo build --release
```
- Dependencies:

  - openssl

  - pkg-config

**OR** 
#### for (musl) --static
- Dependencies
  - Docker setup

```bash
./build.sh
```
## How It Works

#### 1. Getting the changes

- We create a git diff against your PR branch and where you are merging
- All changes that are not relevant are dropped at parsing
- Changes that are relevant and exist within the code are then being passed further

#### 2. Patch Parsing

  

- Entry points:

-  `patchdog/src/binding.rs`

-  `git_parsing/src/patch_parse.rs`

-  `rust_parsing` crate

- Only processes Rust files to avoid broken code.

- Finds all changed functions and prepares them for documentation generation.

  

#### 3. Interface

  

-  `rust_parsing` crate exposes:

-  `RustItemParser` and `RustParser` traits for parsing rust files.

-  `Files` and `FileExtractor` traits for file operations.

- Error propagation in `error.rs`

-  `patchdog`, as entry point:

- methods to interact with other code in the project

- such as sending request with `call()`

- processing the responses with match_request_response

- fallback_repair to autocorrect the broken JSON - attempts to strip the JSON, that the LLM could've returned after serde failed until it returns any viable result, elsewise we parse it with REGEX,

- in `binding.rs`: grouping methods, all public methods are moved to the top of the file

-  `git_parsing` contains a few methods, to sort relevant changes from all hunks

-  `gemini` provides all the necessary interface to prepare a response, collect it into a structure and process the result

  

#### 4. Data Flow

  

- Functions are stored as `SingleFunctionData` objects.

-  `metadata` is excluded from serialization to save LLM tokens and reduce hallucinations.

- Functions are grouped into `MappedRequest` batches, limited by `tokens_per_min`

(default: `250_000` from `config.yaml`).

- Batches are further grouped into `WaitForTimeout` collections, limited by `request_per_min`

(default: `10` for Gemini API).

  

#### 5. LLM Interaction

  

- Model responses are validated by UUID.

- Broken answers trigger recursive retries until all requests succeed.

#### 6. Result writing
- When quantity of responses matches with requests, the requests are written simultaneously