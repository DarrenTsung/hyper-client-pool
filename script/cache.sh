#!/usr/bin/env bash

# Fail the build on any failed command
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

  # if CI_BRANCH cache does not exist, use fallback branch
  if aws s3 ls "s3://onesignal-cache/hyper-client-pool/$CI_BRANCH" 2>&1 | grep -q 'NoSuchBucket'; then
    aws s3 sync cache s3://onesignal-cache/hyper-client-pool/master
  else
    aws s3 sync cache s3://onesignal-cache/hyper-client-pool/$CI_BRANCH
  fi
else
  aws s3 sync cache s3://onesignal-cache/hyper-client-pool/$CI_BRANCH
fi
