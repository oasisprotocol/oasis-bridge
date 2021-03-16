#!/bin/sh -eux
BUILD_NUMBER=3935
OASIS_NODE_ARTIFACT=https://buildkite.com/organizations/oasisprotocol/pipelines/oasis-core-ci/builds/3935/jobs/f3463f8c-2d2c-4381-a115-5d1f34b9ff44/artifacts/b4fa89d0-2712-4591-a5d5-e70efea9db3a
OASIS_NET_RUNNER_ARTIFACT=https://buildkite.com/organizations/oasisprotocol/pipelines/oasis-core-ci/builds/3935/jobs/f3463f8c-2d2c-4381-a115-5d1f34b9ff44/artifacts/bdfa13dc-d111-4860-9823-a53cea0cb218
OASIS_CORE_RUNTIME_LOADER_ARTIFACT=https://buildkite.com/organizations/oasisprotocol/pipelines/oasis-core-ci/builds/3935/jobs/0de25057-fa8c-46be-8fe1-bec554e6bdf3/artifacts/466a2eaa-79be-41a0-9430-0e81c86bc3dd
SIMPLE_KEYMANAGER_ARTIFACT=https://buildkite.com/organizations/oasisprotocol/pipelines/oasis-core-ci/builds/3935/jobs/9fb4239c-3421-41a1-b44e-3dd7303b6755/artifacts/8d1b5c41-d0bc-481c-a1e8-116eb7aed574

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
if [ ! -e ../../../target/debug/oasis-bridge-runtime ]; then
    (
        cd ../../..
        cargo build -p oasis-bridge-runtime
    )
fi

mkdir -p /tmp/oasis-net-runner-bridge
"./untracked/oasis-net-runner-$BUILD_NUMBER" \
    --fixture.default.node.binary "untracked/oasis-node-$BUILD_NUMBER" \
    --fixture.default.runtime.binary ../../../target/debug/oasis-bridge-runtime \
    --fixture.default.runtime.loader "untracked/oasis-core-runtime-loader-$BUILD_NUMBER" \
    --fixture.default.keymanager.binary "untracked/simple-keymanager-$BUILD_NUMBER" \
    --basedir /tmp/oasis-net-runner-bridge \
    --basedir.no_temp_dir
