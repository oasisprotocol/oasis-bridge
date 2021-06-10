#!/bin/sh -eux

TESTS_DIR=../../../tests
. "$TESTS_DIR/consts.sh"
. "$TESTS_DIR/paths.sh"

mkdir -p /tmp/oasis-net-runner-bridge
"$TEST_NET_RUNNER" \
    --fixture.default.node.binary "$TEST_NODE_BINARY" \
    --fixture.default.runtime.binary ../../../target/debug/oasis-bridge-runtime \
    --fixture.default.runtime.loader "$TEST_RUNTIME_LOADER" \
    --fixture.default.keymanager.binary "$TEST_KM_BINARY" \
    --basedir /tmp/oasis-net-runner-bridge \
    --basedir.no_temp_dir
