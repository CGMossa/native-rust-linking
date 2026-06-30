# Linking to OpenSSL

How to inspect what `openssl-sys` pulls into the link, on macOS and Ubuntu (arm64).

## Scaffolding

```sh
cargo new --lib linking_to_openssl
cd linking_to_openssl
cargo add openssl-sys
```

`cargo new --lib` gives a `src/lib.rs`. The library is necessary in order for `cargo` to do the resolution of linker settings.

### Example: using `openssl`

The `openssl-sys` build script always emits its link settings (verify with
`cat target/debug/build/openssl-sys-*/output`):

```
cargo:rustc-link-search=native=/opt/homebrew/opt/openssl@3/lib
cargo:rustc-link-lib=dylib=ssl
cargo:rustc-link-lib=dylib=crypto
```

But if no code references an `openssl-sys` symbol, rustc dead-strips the crate
and drops its `-l` flags, so `-lssl`/`-lcrypto` never reach the link. The
`-L` search path leaks through regardless. Verified: a bin that ignores
`openssl-sys` links with `-L .../openssl@3/lib` only; adding one symbol
reference brings back `-lssl -lcrypto`. Reference one symbol:

```rust
// src/lib.rs
pub fn openssl_version() -> std::os::raw::c_ulong {
    unsafe { openssl_sys::OpenSSL_version_num() }
}
```

`--print native-static-libs` emits its note for `staticlib` only. Verified
across crate types (`rustc --print native-static-libs --crate-type <T>
probe.rs --out-dir .`): `bin`, `rlib`, `cdylib` produce no note; only
`staticlib` does. Add the crate type:

```toml
# Cargo.toml
[lib]
crate-type = ["staticlib", "rlib"]
```

## Commands

```sh
# Linker invocation (full cc/ld command line)
RUSTFLAGS="--print link-args" cargo build

# Native libs a consumer must link against the static lib
RUSTFLAGS="--print native-static-libs" cargo build
```

`cargo clean` between runs: the prints only appear on the link step, which is
skipped when the artifact is already up to date.

## Passing the output to a file

`rustc --print` takes an optional `=FILE` (`rustc --help`: `--print <INFO>[=<FILE>]`):

```sh
RUSTFLAGS="--print native-static-libs=libs.txt" cargo build
RUSTFLAGS="--print link-args=link.txt" cargo build
```

## Results

macOS arm64, OpenSSL 3.6.2 (Homebrew); Ubuntu 24.04 arm64, OpenSSL 3.0.13.

### `--print link-args` (OpenSSL-relevant flags)

| Platform | Flags |
|----------|-------|
| macOS    | `-lssl -lcrypto` + `-L /opt/homebrew/opt/openssl@3/lib` |
| Ubuntu   | `-lssl -lcrypto` (no `-L`: OpenSSL is on the default search path `/usr/lib/aarch64-linux-gnu`) |

### `--print native-static-libs`

| Platform | `native-static-libs:` |
|----------|------------------------|
| macOS    | `-lssl -lcrypto -liconv -lSystem -lc -lm` |
| Ubuntu   | `-lssl -lcrypto -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc` |

`-lssl -lcrypto` is identical on both. The trailing system libs differ by
platform: macOS pulls `-liconv -lSystem`, Linux pulls
`-lgcc_s -lutil -lrt -lpthread -ldl`.

## What each platform links to directly

The trailing libs above are the platform's own baseline, independent of
OpenSSL: they are what the Rust `std` links against on that target. Get the
target triples from `rustc --print target-list`, then probe each with a
trivial `staticlib` (no target build needed beyond `rustup target add <t>`):

```sh
echo 'pub fn f(){}' > probe.rs
rustc --print native-static-libs --target <triple> --crate-type staticlib probe.rs --out-dir .
```

Triples relevant to R packages (R on Windows uses the GNU/Rtools toolchain, so
`-pc-windows-gnu`, never `-msvc`):

| Target triple | Default `native-static-libs:` |
|---------------|-------------------------------|
| `aarch64-apple-darwin`      | `-lSystem -lc -lm` |
| `x86_64-apple-darwin`       | `-lSystem -lc -lm` |
| `x86_64-unknown-linux-gnu`  | `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc` |
| `aarch64-unknown-linux-gnu` | `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc` |
| `x86_64-pc-windows-gnu`     | `-lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp` |

Add `-lssl -lcrypto` on top of these once OpenSSL is referenced. The two
Apple targets are identical; the two Linux targets are identical.

## Older toolchains (testing with 1.81.0)

This crate is `edition = "2024"`, which needs Cargo >= 1.85. Cargo 1.81.0
refuses to parse the manifest (`feature edition2024 is required`). To test the
linking behavior on 1.81.0, set `edition = "2021"` first:

```sh
rustup toolchain install 1.81.0 --profile minimal
sed -i.bak 's/edition = "2024"/edition = "2021"/' Cargo.toml
RUSTFLAGS="--print native-static-libs" rustup run 1.81.0 cargo build
```

The linking output is identical to current stable: edition affects the
language frontend, not linking. (Revert with `mv Cargo.toml.bak Cargo.toml`.)

## Build time: vendored vs system

`openssl-sys` defaults to linking the system OpenSSL (dynamic). The `vendored`
feature instead pulls `openssl-src` and compiles OpenSSL from C source, then
links it statically (needs `perl` + a C compiler):

```sh
cargo add openssl-sys --features vendored
```

Clean-build wall time (macOS arm64, 14 cores, registry warm, `cargo clean`
between runs):

| Build | `cargo build` (clean) |
|-------|-----------------------|
| system (dynamic)   | ~1.2 s |
| vendored (static)  | ~15 s  |

Vendored is ~11x slower to build because it compiles all of OpenSSL. Use it
for portable static binaries; skip it when the system OpenSSL is fine.

## Building on Ubuntu arm64 (Docker)

Ubuntu's apt `rustc` is too old for edition 2024, so install current stable
via rustup. Build into a separate target dir to keep the host `target/` clean.

```sh
docker run --rm --platform linux/arm64 \
  -v "$PWD":/work:ro ubuntu:24.04 bash -c '
    set -e
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq && apt-get install -y -qq curl gcc pkg-config libssl-dev
    curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    . "$HOME/.cargo/env"
    export CARGO_TARGET_DIR=/tmp/t
    cd /work
    RUSTFLAGS="--print native-static-libs" cargo build 2>&1 | grep native-static-libs:
  '
```
