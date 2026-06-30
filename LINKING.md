# Native Rust linking

How to extract the native link settings from a Rust crate, and why that
matters when a foreign build system (such as R's) performs the final link.

## 1. Cargo: a `staticlib` to get the build settings

```sh
cargo new --lib linking_to_openssl
cd linking_to_openssl
```

<!-- verified across crate types: bin, rlib, cdylib print no note; only staticlib does -->
To read the linker settings back out of the build, compile a **`staticlib`**.
`rustc --print native-static-libs` emits its note for `staticlib` only
(`rustc --print native-static-libs --crate-type <T> probe.rs --out-dir .`):
`bin`, `rlib`, and `cdylib` produce no note; only `staticlib` does. So a plain
`--lib` is not enough on its own:

```toml
# Cargo.toml
[lib]
crate-type = ["staticlib", "rlib"]
```

The probe, and exactly what each crate type prints:

```console
$ echo 'pub fn f(){}' > probe.rs   # 'fn main(){}' for the bin case
$ rustc --print native-static-libs --crate-type bin       probe.rs --out-dir .
$ rustc --print native-static-libs --crate-type rlib      probe.rs --out-dir .
$ rustc --print native-static-libs --crate-type cdylib    probe.rs --out-dir .
$ rustc --print native-static-libs --crate-type staticlib probe.rs --out-dir .
note: native-static-libs: -lSystem -lc -lm
```

Only the `staticlib` invocation prints a `note:`; the other three print
nothing.

Commands:

```sh
# The native libs a consumer must link when linking this static library
RUSTFLAGS="--print native-static-libs" cargo build

# The full linker invocation (cc/ld command line)
RUSTFLAGS="--print link-args" cargo build
```

These prints only happen on the link step, which cargo skips when the artifact
is up to date. Run `cargo clean` between runs.

Send the output to a file. `rustc --print` takes an optional `=FILE`
(`rustc --help`: `--print <INFO>[=<FILE>]`), or just redirect stdout:

```sh
RUSTFLAGS="--print native-static-libs=libs.txt" cargo build
RUSTFLAGS="--print link-args" cargo build > link.txt 2>&1
```

## 2. Why this matters for native R packages

When you ship a Rust library inside an R package, **R's build system performs
the final link**, not cargo. The Rust side is compiled to a `staticlib`
(`libfoo.a`), and `R CMD INSTALL` links it into the package shared object. R
therefore needs the list of native libraries the static archive depends on.

You pass that list to R through `src/Makevars` (and `Makevars.win` on
Windows) via `PKG_LIBS`. The list is exactly what `--print
native-static-libs` reports:

```make
# src/Makevars
PKG_LIBS = -L$(CARGO_TARGET)/release -lfoo -lssl -lcrypto <platform libs...>
```

If those libraries are missing from `PKG_LIBS`, the package fails to link.
This is why getting the `native-static-libs` list right is the whole game for
a native R package.

## 3. What Rust links to

The baseline `native-static-libs` for a target is what the Rust `std` itself
links against, before any dependency. List targets with `rustc --print
target-list`, then probe each with a trivial `staticlib` (only `rustup target
add <triple>` is needed, no external SDK):

```console
$ echo 'pub fn f(){}' > probe.rs
$ rustc --print native-static-libs --target x86_64-pc-windows-gnu --crate-type staticlib probe.rs --out-dir .
note: native-static-libs: -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp
```

Triples relevant to R packages (R on Windows uses the GNU/Rtools toolchain, so
`-pc-windows-gnu`, never `-msvc`):

| Target triple | Baseline `native-static-libs:` |
|---------------|--------------------------------|
| `aarch64-apple-darwin`      | `-lSystem -lc -lm` |
| `x86_64-apple-darwin`       | `-lSystem -lc -lm` |
| `x86_64-unknown-linux-gnu`  | `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc` |
| `aarch64-unknown-linux-gnu` | `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc` |
| `x86_64-pc-windows-gnu`     | `-lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp` |

The two Apple targets are identical; the two Linux targets are identical.
Anything a dependency needs is added on top of these.

## 4. Adding the OpenSSL dependency

```sh
cargo add openssl-sys
```

The `openssl-sys` build script always emits its link settings (see
`target/debug/build/openssl-sys-*/output`):

```
cargo:rustc-link-search=native=/opt/homebrew/opt/openssl@3/lib
cargo:rustc-link-lib=dylib=ssl
cargo:rustc-link-lib=dylib=crypto
```

But if no code references an `openssl-sys` symbol, rustc dead-strips the crate
and drops its `-l` flags, so `-lssl`/`-lcrypto` never reach the link (the `-L`
search path leaks through regardless). The openssl-relevant tokens out of
`RUSTFLAGS="--print link-args" cargo build` on macOS, both ways:

```console
$ # main.rs = fn main() {}  -- ignores openssl-sys
"-L" "/opt/homebrew/opt/openssl@3/lib"

$ # main.rs references openssl_sys::OpenSSL_version_num()
"-lssl"
"-lcrypto"
"-L" "/opt/homebrew/opt/openssl@3/lib"
```

So reference one symbol:

```rust
// src/lib.rs
pub fn openssl_version() -> std::os::raw::c_ulong {
    unsafe { openssl_sys::OpenSSL_version_num() }
}
```

The full final link line then, trimmed (the std rlibs and `.o` inputs elided
as `<objects>`):

```console
$ RUSTFLAGS="--print link-args" cargo build
env ... "cc" <objects> "-lssl" "-lcrypto" "-liconv" "-lSystem" "-lc" "-lm" \
  "-arch" "arm64" "-mmacosx-version-min=11.0.0" \
  "-L" "/opt/homebrew/opt/openssl@3/lib" \
  "-o" "target/debug/deps/linking_to_openssl-<hash>" "-Wl,-dead_strip" "-nodefaultlibs"
```

### Results (`native-static-libs`, system OpenSSL)

| Platform | `native-static-libs:` |
|----------|------------------------|
| macOS arm64 (OpenSSL 3.6.2, brew) | `-lssl -lcrypto -liconv -lSystem -lc -lm` |
| Ubuntu 24.04 arm64 (OpenSSL 3.0.13) | `-lssl -lcrypto -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc` |
| `x86_64-pc-windows-gnu` (vendored) | `-lgdi32 -luser32 -lcrypt32 -lws2_32 -ladvapi32 -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp` |

The raw notes, as printed by `RUSTFLAGS="--print native-static-libs" cargo
build`:

```console
# macOS arm64 (system OpenSSL)
note: native-static-libs: -lssl -lcrypto -liconv -lSystem -lc -lm
# Ubuntu 24.04 arm64 (system OpenSSL)
note: native-static-libs: -lssl -lcrypto -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc
# x86_64-pc-windows-gnu (vendored OpenSSL)
note: native-static-libs: -lgdi32 -luser32 -lcrypt32 -lws2_32 -ladvapi32 -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp
```

On macOS/Ubuntu (dynamic system OpenSSL) `-lssl -lcrypto` is constant and the
trailing libs are the platform baseline from section 3. The Windows note is
what the CI writes to `results/` (see below), and the local cross-compile
prints the same set.

The Windows row is **vendored** (the local cross-compile and the CI both build
this target this way). Note there is no `-lssl`/`-lcrypto`: with a static
(vendored) OpenSSL, rustc bundles `libssl.a`/`libcrypto.a` *into* the output
`.a`, so the note lists only the Win32 libraries OpenSSL still needs at link
time (`crypt32`, `ws2_32`, `advapi32`, `user32`, `gdi32`) on top of the
std baseline. That full set is what goes into `Makevars.win` `PKG_LIBS`.

### Reproducing on each platform

**Ubuntu arm64 (Docker).** Ubuntu's apt `rustc` is too old for edition 2024,
so install current stable via rustup; build into a separate target dir to keep
the host `target/` clean:

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

**Windows.** See `.github/workflows/windows-linking.yml`. It builds the
`x86_64-pc-windows-gnu` target with the `vendored` feature and commits the
prints to `results/`. Cross-compile the same target locally (needs
`mingw-w64`) and the flags agree:

```sh
rustup target add x86_64-pc-windows-gnu   # plus mingw-w64 for the linker + vendored build
RUSTFLAGS="--print native-static-libs" \
  cargo build --target x86_64-pc-windows-gnu --features openssl-sys/vendored
```

**Older toolchains (1.81.0).** This crate is `edition = "2024"`, which needs
Cargo >= 1.85; Cargo 1.81.0 refuses the manifest. Set `edition = "2021"` to
test linking on it (linking is edition-independent, output is identical):

```sh
rustup toolchain install 1.81.0 --profile minimal
sed -i.bak 's/edition = "2024"/edition = "2021"/' Cargo.toml
RUSTFLAGS="--print native-static-libs" rustup run 1.81.0 cargo build
```

## 5. Benchmark: system vs vendored

`openssl-sys` links the **system** OpenSSL by default (dynamic, fast). The
`vendored` feature instead pulls `openssl-src` and compiles all of OpenSSL
from C source, then links it statically:

```sh
cargo add openssl-sys --features vendored
```

Clean-build wall time (macOS arm64, 14 cores, registry warm, `cargo clean`
between runs):

| Build | `cargo build` (14 cores) | `cargo build -j1` |
|-------|--------------------------|-------------------|
| system (dynamic) | ~1.2 s | ~3.2 s |
| vendored (static) | ~15 s | ~42 s |

Vendoring is ~11x slower multi-core and ~13x single-core, because it rebuilds
all of OpenSSL. The lesson: presenting the **system dependency** at build time
(and passing its link settings on, e.g. to R via `Makevars`) keeps compilation
cheap. Vendoring is a convenience for portability, not a substitute for having
the system library available.
