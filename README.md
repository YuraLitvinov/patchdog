Requires .env file to function. Insert your key, specify input and from there everything should work.
start.sh should be the entry point as it creates project_files.json file that is necessary for proper operation. 
The script is not bound to only *.rs files, but with a simple edit can insert any type of file into the notation.
Currently, this .json is necessary, as it provides the project with necessary input. Potentially, it could be expanded to include code blocks that were changed as per git.

