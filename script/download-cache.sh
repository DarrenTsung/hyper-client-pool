#!/usr/bin/env bash

# https://www.gnu.org/software/bash/manual/html_node/The-Set-Builtin.html
# -e => Exit on error instead of continuing
set -e

CACHE_DIR=/deploy/cache

# ensure that the cache directory exists
mkdir -p $CACHE_DIR

# This command will fail if no such bucket, so unset 'e' (exit on error) for this part
set +e
aws s3 ls "s3://onesignal-cache/$CI_REPO_NAME/$CI_BRANCH" &> .output
set -e

# if CI_BRANCH cache does not exist, use fallback branch
if cat .output | grep -q 'NoSuchBucket'; then
  echo "No such bucket, using master cache.."
  # aws s3 sync $CACHE_DIR s3://onesignal-cache/$CI_REPO_NAME/master
else
  echo "Using $CI_BRANCH cache.."
  # aws s3 sync $CACHE_DIR s3://onesignal-cache/$CI_REPO_NAME/$CI_BRANCH
fi
