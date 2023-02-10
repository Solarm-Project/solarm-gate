#!/usr/bin/env bash

set -ex

MAX_JOBS="${MAX_JOBS:=1}"

./usr/src/tools/scripts/bldenv ./env/aarch64 "cd usr/src/; make -j ${MAX_JOBS} bldtools sgs"

cp /opt/solarm/lib/gcc/aarch64-solaris2.11/10.3.0/libgcc.a proto/root_aarch64/usr/lib/

for lib in crt ssp_ns libc libm_aarch64 libmd libmp libnsl libsocket libkstat librt libpthread ; do
  ./usr/src/tools/scripts/bldenv ./env/aarch64 "cd usr/src/lib/${lib}; make -j ${MAX_JOBS} install"
done

./usr/src/tools/scripts/bldenv ./env/aarch64 "cd usr/src/cmd/sgs/libdl; make -j ${MAX_JOBS} install"
  
rm proto/root_aarch64/usr/lib/libgcc.a

sleep 10

