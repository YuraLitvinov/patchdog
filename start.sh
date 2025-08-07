
#!/bin/bash
#May be useful for running locally and debugging depending on the chosen alias
git diff main HEAD > base_head.patch
cargo patchdog-debug
