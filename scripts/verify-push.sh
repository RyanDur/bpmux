#!/bin/bash
set -euo pipefail

branch=$(git branch --show-current)
local_sha=$(git rev-parse HEAD)
remote_sha=$(git ls-remote origin "refs/heads/$branch" | cut -f1)

if [ "$local_sha" != "$remote_sha" ]; then
    echo "FAIL: push did not land — local $branch is $local_sha but origin/$branch is ${remote_sha:-absent}."
    echo "The push exit code lied (SIGPIPE, hook short-circuit, or network drop). Push again and re-verify."
    exit 1
fi
echo "Verified: origin/$branch == $local_sha"
