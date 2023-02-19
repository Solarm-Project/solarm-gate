#!/usr/bin/env bash

set -ex

MAX_JOBS="${MAX_JOBS:=1}"
CROSS="/opt/solarm"
SYSROOT="${CROSS}/sysroot"

SRC_DIR=$PWD

cd ../illumos-gate

ls -alh
./usr/src/tools/scripts/bldenv ./env/aarch64 "cd usr/src/; make -j ${MAX_JOBS} bldtools sgs"

cd ../nss_build

ILLUMOS_PATH=$(realpath ../illumos-gate/usr/src)

NSS_BUILD_PATH=$(realpath ../)

PROTO_DIR_REL="../../proto/opt/solarm/sysroot"

mkdir -p $PROTO_DIR_REL

PROTO_DIR=$(realpath $PROTO_DIR_REL)

export NATIVE_MACH=i386 \
	    MACH=aarch64 \
	    SRC="${ILLUMOS_PATH}" \
	    NSS_BASE="${SRC_DIR}" \
	    NSS_BUILD="${NSS_BUILD_PATH}" \
	    ONBLD_TOOLS="${CROSS}" \
	    ROOT="${ILLUMOS_PATH}/proto/root_aarch64" \
	    aarch64_PRIMARY_CC="gcc10,${CROSS}/bin/aarch64-unknown-solaris2.11-gcc,gnu" \
	    aarch64_SYSROOT="${SYSROOT}"; \
			make -j $MAX_JOBS -e install_h && \
			make -j $MAX_JOBS -e install