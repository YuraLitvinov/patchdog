#!/bin/bash
set -euo pipefail


#Set environment variables
PR_NUMBER=$(jq -r .issue.number "$GITHUB_EVENT_PATH")
PR_JSON=$(gh pr view "$PR_NUMBER" --json headRefName,baseRefName,author,url)
HEAD_BRANCH=$(jq -r '.headRefName'  <<< "$PR_JSON")
BASE_BRANCH=$(jq -r '.baseRefName'  <<< "$PR_JSON")
ASSIGNEE=$(jq -r '.author.login'  <<< "$PR_JSON")

#Configure user
git config user.email "$COMMIT_EMAIL" 
git config user.name "$COMMIT_NAME"

git fetch origin "$HEAD_BRANCH":"$HEAD_BRANCH"
git switch "$HEAD_BRANCH"

#Test if PR contains conflict and abort any further actions
git merge --no-commit --no-ff origin/"$BASE_BRANCH" || exit 1
if [ -f .git/MERGE_HEAD ]; then
  git merge --abort
fi



#Download and run latest release
curl -L -o patchdog-linux-x86_64 https://github.com/YuraLitvinov/patchdog/releases/latest/download/patchdog-linux-x86_64
chmod +x patchdog-linux-x86_64
git diff origin/$BASE_BRANCH...origin/$HEAD_BRANCH > base_head.patch
./patchdog-linux-x86_64 --file-patch base_head.patch
#Cleanup artifacts
rm base_head.patch && rm patchdog-linux-x86_64

#Create a unique pull request
PATCHDOG_BRANCH="patchdog-$(date +%s)"
git switch -c "$PATCHDOG_BRANCH"
git add . 

if ! git diff --cached --quiet; then
    git commit -m "Patchdog-included changes for $HEAD_BRANCH"
    git push origin "$PATCHDOG_BRANCH"

    # Authenticate GH CLI and create PR
    echo "$GH_TOKEN" | gh auth login --with-token
    gh pr create \
        --title "Patchdog merge into $HEAD_BRANCH" \
        --body "PR initialized by patchdog" \
        --head "$PATCHDOG_BRANCH" \
        --base "$HEAD_BRANCH" \
        --assignee "$ASSIGNEE" \
        --draft
fi