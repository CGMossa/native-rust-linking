# corpus survey: what extendr packages actually link

Evidence from the [extendr-universe](https://github.com/extendr/awesome-extendr)
corpus: 52 extendr-powered R packages cloned locally and scanned for their
declared and resolved native dependencies (`DESCRIPTION` `SystemRequirements`,
`src/rust/Cargo.lock`, `src/Makevars*`, `configure*`). This is the empirical
backing for the registry: it shows how often a Rust R package needs a
Rtools-supplied system library at all, and what it does instead.

## Headline

- **52/52** declare only `Cargo` + `rustc` as their build requirement (some add
  `xz`, used to unpack vendored-crate tarballs, not to link anything).
- **2/52** link a real external system library:
  - `arcgisplaces` links **OpenSSL** (see [`openssl.md`](openssl.md)).
  - `ggsql` links **ODBC** (`pkg-config --libs odbc` in `tools/config.R`, with a
    plain `-lodbc` fallback; `-lodbc32` on Windows).
- The other 50 contribute only the Rust std baseline, plus, where they need
  crypto/compression, code they bring themselves (pure-Rust crates or vendored
  C/C++ compiled from source). No Rtools `pkg-config` lookup, no installed lib.

So the `openssl.md` conclusion (an extendr R package should avoid leaning on a
Rtools system library and instead carry its dependency in-tree) is not a
preference, it is what the ecosystem already does at a rate of 50 to 2.

## The standard Windows link line

48 of the 52 packages ship the **exact same** `Makevars.win` line:

```make
PKG_LIBS = -lws2_32 -ladvapi32 -luserenv -lbcrypt -lntdll
```

This is the rextendr template default. It is cargo's `--print native-static-libs`
note for a pure-Rust `x86_64-pc-windows-gnu` staticlib (`ws2_32` sockets,
`advapi32`/`userenv` Win32, `bcrypt` for `ring`/`getrandom` CNG, `ntdll`),
written into the Makevars by hand because R, not cargo, runs the final link
(see [`../LINKING.md`](../LINKING.md) §2). Both `arcgisplaces` and `pdfsigner`
literally run `RUSTFLAGS=--print=native-static-libs` in their build recipe to
derive it. The technique LINKING.md documents is the ecosystem norm, not an
exception.

Deviations from the baseline are exactly the packages with an extra dependency:

| `Makevars.win` `PKG_LIBS` adds | package | why |
|---|---|---|
| `-lcrypt32 -lcrypto -lsecur32 -loleaut32 -lmingw32` | arcgisplaces | OpenSSL + schannel (native-tls) |
| `-lstdc++ -static-libstdc++` | freestiler, roxigraph | vendored C++ (duckdb, rocksdb) |
| `-lole32 -lstdc++` | tinyimg | vendored C (image codecs) |
| `-ldbghelp -lrstrtmgr -lodbc32 -lstdc++` | ggsql | duckdb C++ + ODBC |

## Two ways to avoid a system library

The packages that do real crypto/compression overwhelmingly pick one of two
strategies, both of which keep the dependency out of Rtools' hands:

**1. Pure-Rust reimplementation** (no C, no system lib, nothing to link):

| replaces | crate | packages |
|---|---|---|
| OpenSSL/TLS | `rustls` + `ring` | arcgisplaces, freestiler, ggsql, gtfsrealtime, pdfsigner |
| zlib | `zlib-rs` / `libz-rs-sys` | freestiler, ggsql, gtfsrealtime, unsum |
| bzip2 | `libbz2-rs-sys` | gtfsrealtime |

`ring` links Windows CNG (`bcrypt`), already in the baseline, so a rustls+ring
package adds nothing to `PKG_LIBS`. `pdfsigner` is the clean example: same
domain as `arcgisplaces` (TLS) but `PKG_LIBS = -lpdfsigner` on Unix and only the
baseline on Windows.

**2. Vendored C/C++ compiled from source** (the `cc` crate builds it into the
staticlib; still no installed system lib):

| crate | bundles | packages |
|---|---|---|
| `zstd-sys` | zstd | freestiler, gtfsrealtime, unsum |
| `libduckdb-sys` | DuckDB | freestiler, ggsql |
| `oxrocksdb-sys` | RocksDB | roxigraph |
| `mozjpeg-sys` | mozjpeg | tinyimg |
| `libdeflate-sys` | libdeflate | tinyimg |

These cost build time and a `-lstdc++` for the C++ ones, but the resulting `.a`
is self-contained. `roxigraph` additionally needs `libclang` at build time
(`clang-sys`/bindgen), a build dependency, not a link dependency.

`-sys` crates seen in the corpus that are **not** external libraries:
`windows-sys`, `core-foundation-sys`, `security-framework-sys`,
`system-configuration-sys` (OS API bindings, link always-present system libs),
`js-sys`, `web-sys` (wasm), `linux-raw-sys` (syscall constants). They need
nothing installed. The full crate-by-crate breakdown is in [`crates.md`](crates.md).

## Vendoring discipline

What gets vendored today (full table in [`crates.md`](crates.md)):

- Vendored, no Rust-native and no known Rtools alternative: DuckDB, RocksDB,
  mozjpeg, libdeflate.
- Not vendored, a Rust-native version exists and is used: zlib
  (`flate2`/`zlib-rs`), bzip2 (`libbz2-rs-sys`), TLS (`rustls`+`ring`). No
  `libz-sys`, `bzip2-sys`, or `openssl-src` anywhere; the one OpenSSL user links
  the system lib.
- Vendored despite the platform likely having it: `zstd-sys`, pulled
  transitively (`pmtiles2`/`zip`/`parquet`).

Whether the vendored cases *could* avoid vendoring (Rtools shipping or being
asked to add the lib, a system dependency, a maturing Rust crate) is not obvious
and is tracked, with its routes and blockers, as an open task in
[`crates.md`](crates.md#avoiding-vendoring-open-task).

## The CRT fault line, in the wild

`arcgisplaces` and `pdfsigner` both create an empty `libgcc_eh.a` on Windows:

```make
mkdir -p $(TARGET_DIR)/libgcc_mock
touch $(TARGET_DIR)/libgcc_mock/libgcc_eh.a
```

Rust's `x86_64-pc-windows-gnu` staticlib references `libgcc_eh`, which the
Rtools UCRT toolchain doing R's final link does not provide. The mock satisfies
the reference. This is the UCRT-vs-MSVCRT mismatch from [`openssl.md`](openssl.md)
showing up as a concrete workaround, not a theoretical concern.

## Method

```sh
# in a checkout of extendr-universe (`just setup` populates repos/)
grep -rhiE '^SystemRequirements:' repos/*/DESCRIPTION
grep -rhoE '^name = "[a-z0-9_-]+-sys"' repos/*/src/rust/Cargo.lock
grep -rnE '^PKG_LIBS' repos/*/src/Makevars*
```
