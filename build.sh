
#!/bin/bash
#May be useful for running locally and debugging depending on the chosen alias
#Creates a statically-linked binary built with musl 
#git diff main HEAD > base_head.patch
mkdir build 
docker build --tag 'patchdog' .
container_id=$(docker create 'patchdog')
docker cp $container_id:/app/target/release/patchdog ./build/patchdog