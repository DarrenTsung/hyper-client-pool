#!/usr/bin/env bash

# https://www.gnu.org/software/bash/manual/html_node/The-Set-Builtin.html
# -e => Exit on error instead of continuing
set -e

# Parse which command this is
DOWNLOAD=1
UPLOAD=2

STATE=1
if [ "$1" = "download" ]; then
  STATE=$DOWNLOAD
elif [ "$1" = "upload" ]; then
  STATE=$UPLOAD
else
  echo "Please run with 'download' or 'upload' as first argument!"
  exit 1
fi

if [ "$STATE" = $DOWNLOAD ]; then
  # ensure that the cache directory exists
  mkdir -p cache

  # This command will fail if no such bucket, so unset 'e' (exit on error) for this part
  set +e
  aws s3 ls "s3://onesignal-cache/hyper-client-pool/$CI_BRANCH" &> .output
  set -e

  # if CI_BRANCH cache does not exist, use fallback branch
  if cat .output | grep -q 'NoSuchBucket'; then
    echo "No such bucket, using master cache.."
    # aws s3 sync cache s3://onesignal-build/master
  else
    echo "Using $CI_BRANCH cache.."
    # aws s3 sync cache s3://onesignal-build/$CI_BRANCH
  fi
else
  echo "Uploading to cache.."
  ls cache
  # aws s3 sync cache s3://onesignal-build/$CI_BRANCH
fi
