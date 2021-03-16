#!/bin/sh -eux
if [ ! -e ../../../target/debug/oasis-bridge-runtime ]; then
    (
        cd ../../..
        cargo build -p oasis-bridge-runtime
    )
fi
