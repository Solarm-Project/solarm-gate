#!/usr/bin/env bash

set -ex

MAX_JOBS="${MAX_JOBS:=1}"

./usr/src/tools/scripts/bldenv ./env/aarch64 "cd usr/src/; make -j ${MAX_JOBS} bldtools sgs"
