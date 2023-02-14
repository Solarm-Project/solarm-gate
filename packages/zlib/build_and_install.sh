#!/usr/bin/env bash

set -ex

MAX_JOBS="${MAX_JOBS:=1}"
CROSS="/opt/solarm"
SYSROOT="${CROSS}/sysroot"

env PATH="${CROSS}/bin:$PATH" \
	CC=${CROSS}/bin/aarch64-solaris2.11-gcc \
	AR=${CROSS}/bin/aarch64-solaris2.11-ar \
	RANLIB=${CROSS}/bin/aarch64-solaris2.11-ar \
	LDSHARED="${CROSS}/bin/aarch64-solaris2.11-gcc -shared" \
	CFLAGS="--sysroot=${SYSROOT} -fpic" \
	${UNPACK_DIR}/configure --shared --prefix=${SYSROOT}/usr

gmake

gmake install DESTDIR="$PROTO_DIR"

