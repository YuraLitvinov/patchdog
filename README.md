# Patchdog

**Patchdog** is a development tool that creates a **draft pull request** containing generated documentation for your code changes.  
When the draft PR is approved, it merges into your **main PR** with all included commits.  

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
- The Action itself is free of charge. API rates are determined by Google, or other provider of such service, e.g. OpenAI, Anthropic.
---

## Requirements


### GitHub Actions

- Check out the repository, provide:

	- Github token

	- API key for LLM

	- Configuration file where your personal settings are stored. You may manipulate the prompt as well to get more verbose or compact comments

- You might get the reference for setting up patchdog inside patchdog repository, .github/workflows/patchdog.yml

### Local Development

- Rust toolchain installed.

- .env file containing your secret API key.

- System dependencies:

- openssl

- pkg-config


---

## Build Instructions

### Build on your host system (glibc) 
```bash
cargo build --release
```
**OR** 
### (musl)
```bash
./build.sh
```
# How It Works

  

#### 1. Patch Parsing

  

- Entry points:

-  `patchdog/src/binding.rs`

-  `git_parsing/src/patch_parse.rs`

-  `rust_parsing` crate

- Only processes Rust files to avoid broken code.

- Finds all changed functions and prepares them for documentation generation.

  

#### 2. Interface

  

-  `rust_parsing` crate exposes:

-  `RustItemParser` and `RustParser` traits for parsing rust files.

-  `Files` and `FileExtractor` traits for file operations.

- Error propagation in `error.rs`

-  `patchdog`, as entry point:

- methods to interact with other code in the project

- such as sending request with `call()`

- processing the responses with match_request_response

- fallback_repair to autocorrect the broken JSON,

that the LLM could've returned after serde failed

- in `binding.rs`: grouping methods, all public methods are moved to the top of the file

-  `git_parsing` contains a few methods, to sort relevant changes from all hunks

-  `gemini` provides all the necessary interface to prepare a response, collect it into a structure and process the result

  

#### 3. Data Flow

  

- Functions are stored as `SingleFunctionData` objects.

-  `metadata` is excluded from serialization to save LLM tokens and reduce hallucinations.

- Functions are grouped into `MappedRequest` batches, limited by `tokens_per_min`

(default: `250_000` from `config.yaml`).

- Batches are further grouped into `WaitForTimeout` collections, limited by `request_per_min`

(default: `10` for Gemini API).

  

#### 4. LLM Interaction

  

- Model responses are validated by UUID.

- Broken answers trigger recursive retries until all requests succeed.

#### 5. Result writing
- When quantity of responses matches with requests, the requests are written simultaneously