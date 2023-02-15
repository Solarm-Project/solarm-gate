This Repository contains the sources for all the Tools needed to build the SolARM distribution.
To build the distribution use the build packages from the namespace "developer/distribution"

# Design Goals
- Seperate source code from build space (Workspace)
- interactive change of defintions
- linting of defintions
- be able to step through the whole proces step by step
- define a update workflow and guide the user through it
- be able to view failed steps
- force subtools to use the right diagnostic output places

## Sysroot Packages
** Working **
- Binutils Cross
- Headers and linker (sysroot-base)
- GCC-bootstrap
- illumos-sysroot-libraries
  - crt
  - ssp_ns
  - libc
  - libm (aarch64)
  - libmd
  - libmp
  - libnsl
  - libsocket
  - libkstat
  - librt
  - libpthread
  - libdl
- zlib
- libxml2 (after zlib and sysroot-libraries)
- openssl
- idnkit

** Next **
- gcc-full
- nspr
- nss
- xorriso
- u-boot
- dtc

## illumos-gate
**illumos-gate**

### Kernel Drivers ###
- FreeBSD bHyve ARM effort https://reviews.freebsd.org/D26976

## Forge ##
- Bonsai permissions make Resource name https://dev.bonsaidb.io/release/docs/bonsaidb/core/permissions/bonsai/fn.document_resource_name.html
- https://dev.bonsaidb.io/release/docs/bonsaidb/core/permissions/bonsai/enum.DocumentAction.html
- https://dev.bonsaidb.io/release/guide/administration/permission-statements.html
- https://github.com/async-graphql/examples/blob/6ccaafbad3d2f38ce6347c6699466fbbc017f910/axum/starwars/src/main.rs
