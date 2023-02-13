# SolARM Gate
This Repository contains the sources for all the Tools needed to build the SolARM distribution.
To build the distribution use the build packages from the namespace "developer/distribution"

## Design Goals
- Seperate source code from build space (Workspace)
- interactive change of defintions
- linting of defintions
- be able to step through the whole proces step by step
- define a update workflow and guide the user through it
- be able to view failed steps
- force subtools to use the right diagnostic output places

### Sysroot Packages
** Working **
- Binutils Cross
- Headers and linker (sysroot-base)
- GCC-bootstrap

** Next **
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
- gcc-full
- nspr
- nss
- xorriso
- u-boot
- dtc

### illumos-gate
**illumos-gate**