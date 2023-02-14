#!/usr/bin/env bash

set -ex

MAX_JOBS="${MAX_JOBS:=1}"
CROSS="/opt/solarm"
SYSROOT="${CROSS}/sysroot"

PATH="${CROSS}/bin:$PATH"
CC="gcc --sysroot=${SYSROOT}"
CFLAGS="-I${SYSROOT}/usr/include"
LDFLAGS="-shared -Wl,-z,text,-z,aslr,-z,ignore"
MAKE=gmake
${UNPACK_DIR}/Configure \
	--prefix=${SYSROOT}/usr \
	--api=1.1.1 \
	shared threads zlib enable-ec_nistp_64_gcc_128 no-asm \
	solaris-aarch64-gcc

#	--cross-compile-prefix=aarch64-solaris2.11- \
gmake

gmake install DESTDIR="$PROTO_DIR" 

