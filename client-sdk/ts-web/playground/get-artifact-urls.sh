#!/bin/sh -eu

# Use this on a build number, e.g.
#
#     ./get-artifact-urls.sh 3935
#
# to get the artifact URLs for sample-run-network.sh without having to do a
# ton of right-clicking.

ORGANIZATION=oasisprotocol
PIPELINE=oasis-core-ci
BUILD_NUMBER=$1

type curl >/dev/null
type jq >/dev/null

printf 'BUILD_NUMBER=%s\n' "$BUILD_NUMBER"

BUILD_JSON=$(curl -sf "https://buildkite.com/$ORGANIZATION/$PIPELINE/builds/$BUILD_NUMBER.json")
NODE_JOB_ID=$(printf '%s' "$BUILD_JSON" | jq -r '.jobs[] | select(.name == "Build Go node") | .id')
NODE_ARTIFACTS_JSON=$(curl -sf "https://buildkite.com/organizations/$ORGANIZATION/pipelines/$PIPELINE/builds/$BUILD_NUMBER/jobs/$NODE_JOB_ID/artifacts")
OASIS_NODE_URL=$(printf '%s' "$NODE_ARTIFACTS_JSON" | jq -r '.[] | select(.path == "oasis-node") | .url')
printf 'OASIS_NODE_ARTIFACT=https://buildkite.com%s\n' "$OASIS_NODE_URL"
OASIS_NET_RUNNER_URL=$(printf '%s' "$NODE_ARTIFACTS_JSON" | jq -r '.[] | select(.path == "oasis-net-runner") | .url')
printf 'OASIS_NET_RUNNER_ARTIFACT=https://buildkite.com%s\n' "$OASIS_NET_RUNNER_URL"

RUNTIME_LOADER_JOB_ID=$(printf '%s' "$BUILD_JSON" | jq -r '.jobs[] | select(.name == "Build Rust runtime loader") | .id')
RUNTIME_LOADER_ARTIFACTS_JSON=$(curl -sf "https://buildkite.com/organizations/$ORGANIZATION/pipelines/$PIPELINE/builds/$BUILD_NUMBER/jobs/$RUNTIME_LOADER_JOB_ID/artifacts")
OASIS_CORE_RUNTIME_LOADER_URL=$(printf '%s' "$RUNTIME_LOADER_ARTIFACTS_JSON" | jq -r '.[] | select(.path == "oasis-core-runtime-loader") | .url')
printf 'OASIS_CORE_RUNTIME_LOADER_ARTIFACT=https://buildkite.com%s\n' "$OASIS_CORE_RUNTIME_LOADER_URL"

KEY_MANAGER_RUNTIME_JOB_ID=$(printf '%s' "$BUILD_JSON" | jq -r '.jobs[] | select(.name == "Build key manager runtime") | .id')
KEY_MANAGER_RUNTIME_ARTIFACTS_JSON=$(curl -sf "https://buildkite.com/organizations/$ORGANIZATION/pipelines/$PIPELINE/builds/$BUILD_NUMBER/jobs/$KEY_MANAGER_RUNTIME_JOB_ID/artifacts")
SIMPLE_KEYMANAGER_URL=$(printf '%s' "$KEY_MANAGER_RUNTIME_ARTIFACTS_JSON" | jq -r '.[] | select(.path == "simple-keymanager") | .url')
printf 'SIMPLE_KEYMANAGER_ARTIFACT=https://buildkite.com%s\n' "$SIMPLE_KEYMANAGER_URL"
