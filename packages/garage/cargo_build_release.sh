#!/usr/bin/env bash

set -ex

MAX_JOBS="${MAX_JOBS:=1}"

source $HOME/.cargo/env

cargo build \
	--release \
	--no-default-features \
	--features system-libs,k2v,kubernetes-discovery,metrics,telemetry-otlp,sled

mkdir output
cp $HOME/.cargo/target/release/garage output/garage
chmod +x output/garage
chmod +x garage.ksh
