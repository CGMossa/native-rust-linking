# Linking to OpenSSL

How to inspect what `openssl-sys` pulls into the link, on macOS and Ubuntu (arm64).

## Scaffolding

```sh
cargo new --lib linking_to_openssl
cd linking_to_openssl
cargo add openssl-sys
```

`cargo new --lib` gives a `src/lib.rs`. The `bin` (`src/main.rs`) is optional.

### Make linking real

A crate that never references an `openssl-sys` symbol gets dropped by the
linker, so `-lssl`/`-lcrypto` never appear. Reference one symbol:

```rust
// src/lib.rs
pub fn openssl_version() -> std::os::raw::c_ulong {
    unsafe { openssl_sys::OpenSSL_version_num() }
}
```

`--print native-static-libs` only emits for `staticlib`, never `bin` or plain
`rlib`. Add the crate type:

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

Or just redirect stdout (cargo prints these to stdout):

```sh
RUSTFLAGS="--print native-static-libs" cargo build > out.txt 2>&1
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
