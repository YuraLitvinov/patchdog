# Patchdog
Currently, output is a patch file that you can download as an artifact.
It is planned to be a PR with changes from base to head and utilize context.  
Requires .env file to function on local system or providing secrets to Github Actions. 
Insert your key and from there everything should work.
The script is bound to Rust files, as it uses parsing to avoid unnecessary or broken code. 
The project aims at smooth Github Actions integration, as it can be used to document changes in a repository.
Essentially, this tool manipulates a patch file, finds all changes and creates documentation for them.
