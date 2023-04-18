This instructions are temporary and will later be simplified once we provide libstd and libc precompiled and as target installable via packages. But for now it outlines the process needed to get a standard rust compiler to compile unknown illumos arch combination like `sparcv9-unknown-illumos` `aarch64-unknown-illumos` and `riscv64-unknown-illumos` 

**Note: if your crate needs an external C Library you will have to fiddle with it a bit to properly set the include and cross build directives that crate supports.**

1. Get Rustup via the normal channels (goto https://rustup.rs copy and paste OSX instructions)
2. Install Nightly toolchain
3. print an already existing illumos rustc target and adjust to the target CPU  `rustc +nightly -Z unstable-options --target=aarch64-unknown-linux --print target-spec-json` 
	1. check other OS's spec files for that CPU to see what is OS specific (most) and what is CPU specific
	2. use https://llvm.org/docs/LangRef.html#data-layout to adjust `data-layout` key according to platform (all other OS's) should use the same or they are probably equally bugged
	3. Set llvm-target to `<arch>-unknown-solaris2.11` where arch is the same as used in the `arch` key above
	4. any linker flags that are system independant in `pre-link-args` 
4. Setup Cross compile capable GCC plus LD and illumos gate sysroot as usual
5. Use a path from https://doc.rust-lang.org/cargo/reference/config.html to write either user specific cargo overrides or system based ones.
6. Patch the dependency tree to use fixed libc crate from https://github.com/Toasterson/libc.git
	1. run `cargo update -p libc` to update Cargo.lock
7. export `OPENSSL_DIR` from sysroot if needed
8. Use the invocation to compile your crate `cargo +nightly build -Z build-std --target=/path/to/aarch64-unknown-illumos.json`

Example files:

**aarch64-unknown-illumos.json:**
```json
{
  "arch": "aarch64",
  "data-layout": "e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128",
  "dynamic-linking": true,
  "eh-frame-header": false,
  "frame-pointer": "always",
  "has-rpath": true,
  "is-like-solaris": true,
  "late-link-args": {
    "gcc": [
      "-lc",
      "-lssp"
    ]
  },
  "limit-rdylib-exports": false,
  "linker-is-gnu": false,
  "llvm-target": "aarch64-unknown-solaris2.11",
  "max-atomic-width": 128,
  "os": "illumos",
  "pre-link-args": {
    "gcc": [
      "-std=c99"
    ]
  },
  "supported-sanitizers": [
    "address",
    "cfi"
  ],
  "target-family": [
    "unix"
  ],
  "target-pointer-width": "64"
}
```

**cross compile cargo config for gcc with custom sysroot**

```toml
#[build]
#target-dir = "/export/home/vagrant/.cargo/target" # Set this to instruct cargo to place the compile artifacts into the specified directory (usefull for NFS mounted directories)


[target.aarch64-unknown-illumos]
linker = "/opt/solarm/bin/aarch64-unknown-solaris2.11-gcc" # Path of the gcc binary to use as linker (calls our LD in the background but $LD cannot be set)
rustflags = "-Clink-arg=--sysroot=/opt/solarm/braich" # Any linker args that are usually set in $LDFLAGS go here
```

