#!/usr/bin/env bash

set -ex

git lfs pull origin
git submodule update --init --recursive
ci-tools/unlock_keychain.sh
            # removes all files not under source control.
git clean -d -f -x --exclude=.jenkinsBuildFailed
