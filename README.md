# Patchdog
Creates a draft PR that infers into your main PR with filled in comments, you may approve it and it will appear as 
part of your base branch, hence, your main PR stays open.
You can see it in action here: https://github.com/YuraLitvinov/patchdog/pull/19 commit: b6c8a972d768d6da1a280b7b060629597c0cc160
Requires .env file to function on local system or providing secrets to Github Actions. 
Insert your key and from there everything should work.
The script is bound to Rust files, as it uses parsing to avoid broken code. 
The project aims to be ran as an Actions.
Essentially, this tool manipulates a patch file, finds all changes and creates documentation for them.
You also can build it on your system with cargo build and use locally. 
It's very straight-forward and doesn't do anything you wouldn't expect from it.
    Plans: 
    1. Differentiate between trait functions and normal functions.
    2. Add context to request, so LLM can produce better documentation.
    3. Cleanup existing comments, to replace them with new ones, this step is bound to yml config, so 
    any objects, files or functions that are in the yml config will be skipped.

Build prerequisites: set-up docker for statically-linked musl version, run build.sh or dynamically-linked version, but you must provide all necessary dependecies: 
    openssl, pkg-config
If you choose glibc version for running on your host, you can use cargo build to setup things for you.


Please, do not hesitate to contact me if you run into certain issues while using the provided service, 
although NOTHING IS GUARANTEED: litvinov.yura@gmail.com
