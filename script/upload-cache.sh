#!/usr/bin/env bash

# https://www.gnu.org/software/bash/manual/html_node/The-Set-Builtin.html
# -e => Exit on error instead of continuing
set -e

CACHE_DIR=/deploy/cache

echo "Uploading to cache.."
ls $CACHE_DIR
# aws s3 sync $CACHE_DIR s3://onesignal-cache/$CI_REPO_NAME/$CI_BRANCH
