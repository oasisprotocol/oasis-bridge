# User/Witness Flow Example

**Work in progress, may change as the SDKs evolve.**

## Building the Runtime

To build the runtime, run in the top-level directory:

```
cargo build -p oasis-bridge-runtime
```

## Running a Local Network with the Runtime

After being built the runtime will be available under
`target/debug/oasis-bridge-runtime`. Do not invoke it directly, rather you need
to use the `oasis-net-runner` which is part of Oasis Core to run a local
network.

To make the following instructions easier, you should set the
`OASIS_BRIDGE_RUNTIME_PATH` environment variable to the absolute location of the
runtime binary you built above:

```
export OASIS_BRIDGE_RUNTIME_PATH=/path/to/target/debug/oasis-bridge-runtime
```

Before proceeding, make sure to look at the Oasis Core [prerequisites] required
for running an Oasis Core environment followed by [build instructions] for the
respective environment (non-SGX). The following sections assume that you have
successfully completed the required build steps.

To start a local network with the `oasis-bridge-runtime` run the following in
the directory where you built Oasis Core:

```
./go/oasis-net-runner/oasis-net-runner \
  --fixture.default.node.binary go/oasis-node/oasis-node \
  --fixture.default.runtime.binary ${OASIS_BRIDGE_RUNTIME_PATH} \
  --fixture.default.runtime.loader target/default/debug/oasis-core-runtime-loader \
  --fixture.default.keymanager.binary target/default/debug/simple-keymanager \
  --basedir /tmp/oasis-net-runner-bridge \
  --basedir.no_temp_dir
```

This will start a local Oasis network consisting of multiple validator and
compute nodes. You may need to wait a while for it to become ready.

<!-- markdownlint-disable line-length -->
[prerequisites]: https://docs.oasis.dev/oasis-core/development-setup/build-environment-setup-and-building/prerequisites
[build instructions]: https://docs.oasis.dev/oasis-core/development-setup/build-environment-setup-and-building/building
<!-- markdownlint-enable line-length -->

## Building and Running the Example

To build the example, run in this directory:

```
go build main.go
```

This will generate a `main` binary. To run the example and make it use the
previously started local network do:

```
export OASIS_NODE_GRPC_ADDR=unix:/tmp/oasis-net-runner-bridge/net-runner/network/client-0/internal.sock
export BRIDGE_RUNTIME_ID=8000000000000000000000000000000000000000000000000000000000000000
./main
```

This should output something like the following:

```
ts=2020-12-18T12:36:58.97909075Z level=debug module=user-witness-flow caller=main.go:405 msg="establishing connection" addr=unix:/tmp/oasis-net-runner-bridge/net-runner/network/client-0/internal.sock
ts=2020-12-18T12:36:58.979552821Z level=info module=user-witness-flow caller=main.go:116 side=user msg="submitting lock transaction"
ts=2020-12-18T12:36:58.980442041Z level=debug module=user-witness-flow caller=main.go:261 side=witness msg="seen new block" round=17
ts=2020-12-18T12:37:00.290462913Z level=debug module=user-witness-flow caller=main.go:261 side=witness msg="seen new block" round=18
ts=2020-12-18T12:37:00.291112437Z level=debug module=user-witness-flow caller=main.go:184 side=user msg="seen new block" round=17
ts=2020-12-18T12:37:00.291263295Z level=debug module=user-witness-flow caller=main.go:281 side=witness msg="got event" key="YnJpZGdlAAAAAQ==" value=o2JpZBgqZW93bmVyVQBRG4UxYvnMk+dJ5CRAh8fAB2gIMmZhbW91bnSCQQFA
ts=2020-12-18T12:37:00.291298774Z level=debug module=user-witness-flow caller=main.go:296 side=witness msg="got lock event" id=42 owner=oasis1qpg3hpf3vtuueyl8f8jzgsy8clqqw6qgxgurwfy5 amount="1 <native>"
ts=2020-12-18T12:37:00.291321961Z level=info module=user-witness-flow caller=main.go:322 side=witness msg="submitting witness transaction" id=42
ts=2020-12-18T12:37:00.291398508Z level=debug module=user-witness-flow caller=main.go:184 side=user msg="seen new block" round=18
ts=2020-12-18T12:37:00.292099715Z level=debug module=user-witness-flow caller=main.go:202 side=user msg="got event" key="YnJpZGdlAAAAAQ==" value=o2JpZBgqZW93bmVyVQBRG4UxYvnMk+dJ5CRAh8fAB2gIMmZhbW91bnSCQQFA
ts=2020-12-18T12:37:01.397963944Z level=debug module=user-witness-flow caller=main.go:184 side=user msg="seen new block" round=19
ts=2020-12-18T12:37:01.398436909Z level=info module=user-witness-flow caller=main.go:378 side=witness msg="successfully witnessed events"
ts=2020-12-18T12:37:01.39846736Z level=debug module=user-witness-flow caller=main.go:261 side=witness msg="seen new block" round=19
ts=2020-12-18T12:37:01.39871823Z level=debug module=user-witness-flow caller=main.go:202 side=user msg="got event" key="YnJpZGdlAAAAAw==" value=omJpZBgqZHNpZ3OA
ts=2020-12-18T12:37:01.398749519Z level=debug module=user-witness-flow caller=main.go:217 side=user msg="got witnesses signed event" id=42
ts=2020-12-18T12:37:01.39876237Z level=info module=user-witness-flow caller=main.go:224 side=user msg="got witness signatures" sigs="unsupported value type"
ts=2020-12-18T12:37:01.398784999Z level=info module=user-witness-flow caller=main.go:101 side=user msg=done
ts=2020-12-18T12:37:01.399105431Z level=debug module=user-witness-flow caller=main.go:281 side=witness msg="got event" key="YnJpZGdlAAAAAw==" value=omJpZBgqZHNpZ3OA
ts=2020-12-18T12:37:04.409843652Z level=debug module=user-witness-flow caller=main.go:261 side=witness msg="seen new block" round=20
```
