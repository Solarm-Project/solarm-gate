#!/usr/bin/env bash

set -ex

MAX_JOBS="${MAX_JOBS:=1}"
MAKE=gmake
cd ${UNPACK_DIR}
GOBIN= $MAKE all WHAT=cmd/kube-apiserver GOFLAGS=-v 

ginstall -D -m 0755 -t "$PROTO_DIR/usr/bin/" _output/local/go/bin/*
ginstall -D -m 0644 -t "$PROTO_DIR/lib/svc/manifest/application/kubernetes/" kube-apiserver.xml
ginstall -D -m 0755 -T method.sh "$PROTO_DIR/lib/svc/method/kube-apiserver"

