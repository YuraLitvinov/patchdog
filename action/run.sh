#!/bin/bash
set -e

#Configure user
git config user.email "$COMMIT_EMAIL" 
git config user.name "$COMMIT_NAME"

git fetch origin ${BASE_BRANCH}

#Test if PR contains conflict and abort any further actions
git merge --no-commit --no-ff origin/"$BASE_BRANCH" || exit 1

if [ -f .git/MERGE_HEAD ]; then
  git merge --abort
fi

#Download and run latest release
curl -L -o patchdog-linux-x86_64 https://github.com/YuraLitvinov/patchdog/releases/latest/download/patchdog-linux-x86_64
chmod +x patchdog-linux-x86_64
git diff origin/$BASE_BRANCH...$HEAD_BRANCH > base_head.patch
./patchdog-linux-x86_64 --file-patch base_head.patch
#Cleanup artifacts
rm base_head.patch && rm patchdog-linux-x86_64

#Create a unique pull request
PATCHDOG_BRANCH="patchdog-$(uuidgen)"
git checkout -b "$PATCHDOG_BRANCH"
git add . 

git commit -m "Patchdog-included changes for $HEAD_BRANCH"
git push origin $PATCHDOG_BRANCH
gh pr create \
  --title "Patchdog merge into $HEAD_BRANCH" \
  --body "PR initialized by patchdog" \
  --head "$PATCHDOG_BRANCH" \
  --base "$HEAD_BRANCH" \
  --assignee "$ASSIGNEE" \
  --draft