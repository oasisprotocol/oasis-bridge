#!/bin/sh -eux
. ./consts.sh

./download-artifacts.sh
./build-runtime.sh

mkdir -p /tmp/oasis-net-runner-bridge
"./untracked/oasis-net-runner-$BUILD_NUMBER" \
    --fixture.default.node.binary "untracked/oasis-node-$BUILD_NUMBER" \
    --fixture.default.runtime.binary ../../../target/debug/oasis-bridge-runtime \
    --fixture.default.runtime.loader "untracked/oasis-core-runtime-loader-$BUILD_NUMBER" \
    --fixture.default.keymanager.binary "untracked/simple-keymanager-$BUILD_NUMBER" \
    --basedir /tmp/oasis-net-runner-bridge \
    --basedir.no_temp_dir
