#!/bin/sh -eux
. ./consts.sh

mkdir -p untracked
if [ ! -e "untracked/oasis-node-$BUILD_NUMBER" ]; then
    curl -fLo "untracked/oasis-node-$BUILD_NUMBER" "$OASIS_NODE_ARTIFACT"
    chmod +x "untracked/oasis-node-$BUILD_NUMBER"
fi
if [ ! -e "untracked/oasis-net-runner-$BUILD_NUMBER" ]; then
    curl -fLo "untracked/oasis-net-runner-$BUILD_NUMBER" "$OASIS_NET_RUNNER_ARTIFACT"
    chmod +x "untracked/oasis-net-runner-$BUILD_NUMBER"
fi
if [ ! -e "untracked/oasis-core-runtime-loader-$BUILD_NUMBER" ]; then
    curl -fLo "untracked/oasis-core-runtime-loader-$BUILD_NUMBER" "$OASIS_CORE_RUNTIME_LOADER_ARTIFACT"
    chmod +x "untracked/oasis-core-runtime-loader-$BUILD_NUMBER"
fi
if [ ! -e "untracked/simple-keymanager-$BUILD_NUMBER" ]; then
    curl -fLo "untracked/simple-keymanager-$BUILD_NUMBER" "$SIMPLE_KEYMANAGER_ARTIFACT"
    chmod +x "untracked/simple-keymanager-$BUILD_NUMBER"
fi
