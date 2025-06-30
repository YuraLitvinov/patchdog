if [ -f project_files.json ]; then
    rm project_files.json
    echo project_files.json deleted
fi
echo "[" > project_files.json
          FIRST=1
          while IFS= read -r file; do
            # Add comma between objects if not first
            if [ $FIRST -eq 0 ]; then
              echo "," >> project_files.json
            fi

            echo "\"$file\"" >> project_files.json
            FIRST=0
          done < <(find . -type f -name "*.cs" | sort)
          echo "]" >> project_files.json

#cargo run
