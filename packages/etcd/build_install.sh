#!/usr/bin/env bash

set -ex

MAX_JOBS="${MAX_JOBS:=1}"
MAKE=gmake
cd ${UNPACK_DIR}
gmake build

ginstall -D -m 0755 -t "$PROTO_DIR/usr/bin/" bin/etcd
ginstall -D -m 0755 -t "$PROTO_DIR/usr/bin/" bin/etcdctl
ginstall -D -m 0755 -t "$PROTO_DIR/usr/bin/" bin/etcdutl
ginstall -D -m 0644 -t "$PROTO_DIR/lib/svc/manifest/application/database" etcd.xml 
ginstall -D -m 0644 -t "$PROTO_DIR/etc/ectd" etcd.conf.yaml.sample

