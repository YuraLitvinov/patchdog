Requires .env file to function. Insert your key, specify input and from there everything should work.
The script is bound to Rust files, as it uses parsing to avoid documented and unnecessary code. 
Main goal there is, is to avoid unnecessary dependencies that would carry a lot of overhead and impossible to maintain. 
The project aims at smooth Github Actions integration, as it can be used to document changes in a repository - either dump them
to a LLM or be documented by a human. 
Essentially, this tool manipulates a patch file, checking for current documentation, providing a warning for it's absence and offering to revert to LLM for dynamically joining all of their dependencies and creating documentation for them.
