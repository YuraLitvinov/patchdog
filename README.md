# Patchdog
Currently, output is a patch file that you can download as an artifact.
It is planned to be a PR with changes from base to head and utilize context.  
Requires .env file to function on local system or providing secrets to Github Actions. 
Insert your key and from there everything should work.
The script is bound to Rust files, as it uses parsing to avoid broken code. 
The project aims at smooth Github Actions integration, as it can be used to document changes in a repository.
Essentially, this tool manipulates a patch file, finds all changes and creates documentation for them.
You also can build it on your system with cargo build and use locally. It's very straight-forward and doesn't do 
anything you wouldn't expect from it.
    Plans: 
    1.provide conditionals in form of yml file to avoid directories that do not require changes.
    2.add context to request, so LLM can produce better documentation.
    3.cleanup existing comments, to replace them with new ones, this step is bound to yml config, so 
    any objects, files or functions that are in the yml config will be skipped.