# corpus survey: what extendr packages actually link

Evidence from the [extendr-universe](https://github.com/extendr/awesome-extendr)
corpus: 64 extendr-powered R packages cloned locally and scanned for their
declared and resolved native dependencies (`DESCRIPTION` `SystemRequirements`,
`src/rust/Cargo.lock`, `src/Makevars*`, `configure*`). The set is the curated
awesome-extendr list (most of them on CRAN) plus non-CRAN packages found by
GitHub search, which is what lets the CRAN/non-CRAN contrast below show up. This
is the empirical backing for the registry: how often a Rust R package needs a
provided system library, and what it does instead.

## Headline

- Most packages declare only `Cargo` + `rustc` (a few add `xz`, used to unpack
  vendored-crate tarballs, not to link anything).
- A real external system library that must be **provided or installed** (not just
  vendored from source) shows up in only **3 of 64**:
  - `arcgisplaces` links **OpenSSL** (system; see [`openssl.md`](openssl.md)).
  - `ggsql` links **ODBC** (`pkg-config --libs odbc`, `-lodbc` / `-lodbc32`).
  - `masreml` links **OpenBLAS / LAPACK** (`ndarray-linalg` with
    `openblas-static`/`openblas-system`). R already ships BLAS/LAPACK, so this is
    the most clearly avoidable one.
- The rest contribute only the Rust std baseline plus, where they need
  crypto/compression, code they bring themselves: a pure-Rust crate, or C
  vendored from source.

## CRAN vs non-CRAN: a discipline gradient

The curated (mostly CRAN) packages lean hard on pure-Rust crates and system
libraries; the non-CRAN GitHub packages pull C `-sys` crates and vendor more
freely. The same dependency, opposite choices:

| dependency | CRAN-set choice | non-CRAN choice |
|---|---|---|
| OpenSSL | `arcgisplaces` links the system lib | `masreml` vendors it (`openssl-src`) |
| zlib | `flate2`/`zlib-rs` (pure Rust) | `zr` pulls `libz-sys` (C) |
| bzip2 | `gtfsrealtime`: `libbz2-rs-sys` (pure Rust) | `oxbow`: `bzip2-sys` (C) |
| BLAS | (none in the CRAN set) | `masreml`: `openblas-src` rather than R's BLAS |

CRAN's review and multi-platform build requirements push toward portable,
vendor-free choices; a GitHub-only package faces no such pressure. So "extendr
packages avoid system dependencies" is more precisely "CRAN extendr packages
avoid system dependencies."

## Native C / C++ library crates -> packages

These compile or link an actual C/C++ library; only these can need a
Rtools-supplied lib or duplicate one by vendoring. Full per-crate detail, with
the vendoring verdict and the CI probe of what Rtools already ships, is in
[`crates.md`](crates.md).

| crate | system dep | packages |
|---|---|---|
| `zstd-sys` | zstd | freestiler, gtfsrealtime, unsum, zr |
| `openssl-sys` | OpenSSL | arcgisplaces (system), masreml (vendored) |
| `libduckdb-sys` | DuckDB | freestiler, ggsql |
| `libdeflate-sys` | libdeflate | oxbow, tinyimg |
| `openblas-src` | OpenBLAS/LAPACK | masreml |
| `bzip2-sys` | bzip2 | oxbow |
| `oxrocksdb-sys` | RocksDB | roxigraph |
| `mozjpeg-sys` | mozjpeg | tinyimg |
| `libz-sys` | zlib | zr |
| `clang-sys` (build-time) | libclang | roxigraph; `hftokenizers` declares `libclang`/`llvm-config` |

`-sys` crates that are **not** external libraries (OS-API bindings, link
always-present system libs, need nothing installed): `windows-sys`,
`core-foundation-sys`, `security-framework-sys`, `system-configuration-sys`,
`dirs-sys`, `js-sys`, `web-sys`, `linux-raw-sys`.

## The standard Windows link line

53 of the 64 ship the **exact same** `Makevars.win` line:

```make
PKG_LIBS = -lws2_32 -ladvapi32 -luserenv -lbcrypt -lntdll
```

This is the rextendr template default. It is cargo's `--print native-static-libs`
note for a pure-Rust `x86_64-pc-windows-gnu` staticlib (`ws2_32` sockets,
`advapi32`/`userenv` Win32, `bcrypt` for `ring`/`getrandom` CNG, `ntdll`),
written into the Makevars by hand because R, not cargo, runs the final link (see
[`../LINKING.md`](../LINKING.md) §2). The technique LINKING.md documents is the
ecosystem norm, not an exception. `vdjmatchR` is a variation worth noting: it
computes the same line at build time with `rextendr:::rustc_link_flags()`
instead of hardcoding it.

Deviations from the baseline are exactly the packages with an extra dependency:

| `Makevars.win` `PKG_LIBS` adds | package | why |
|---|---|---|
| `-lcrypt32 -lcrypto -lsecur32 -loleaut32 -lmingw32` | arcgisplaces | OpenSSL + schannel (native-tls) |
| `-lstdc++ -static-libstdc++` | freestiler, roxigraph | vendored C++ (duckdb, rocksdb) |
| `-lole32 -lstdc++` | tinyimg | vendored C (image codecs) |
| `-ldbghelp -lrstrtmgr -lodbc32 -lstdc++` | ggsql | duckdb C++ + ODBC |

## Two ways to avoid a system library

The packages that do real crypto/compression mostly pick one of two strategies,
both keeping the dependency out of Rtools' hands.

**1. Pure-Rust reimplementation** (no C, no system lib, nothing to link). These
link nothing beyond the OS baseline (`ring` uses Windows CNG `bcrypt`, already in
the baseline):

| crate | replaces | packages |
|---|---|---|
| `rustls` + `ring` | OpenSSL / TLS | arcgisplaces, freestiler, ggsql, gtfsrealtime, oxbow, pdfsigner |
| `zlib-rs` / `libz-rs-sys` | zlib (`libz-sys`) | freestiler, ggsql, gtfsrealtime, oxbow, tynding, unsum |
| `miniz_oxide` | zlib/deflate (`flate2` backend) | 13 packages |
| `libbz2-rs-sys` | bzip2 (`bzip2-sys`) | gtfsrealtime, oxbow |

**2. Vendored C/C++ compiled from source** (the `cc` crate builds it into the
staticlib): `zstd-sys`, `libduckdb-sys`, `oxrocksdb-sys`, `mozjpeg-sys`,
`libdeflate-sys`, `bzip2-sys`. Self-contained `.a`, but see the vendoring task in
[`crates.md`](crates.md#avoiding-vendoring-open-task): the CI probe shows Rtools
already ships several of these (`libzstd`, `libdeflate`, `zlib`), so on Windows
the bundled copy duplicates an available library.

## The CRT fault line, in the wild

`arcgisplaces` and `pdfsigner` both create an empty `libgcc_eh.a` on Windows:

```make
mkdir -p $(TARGET_DIR)/libgcc_mock
touch $(TARGET_DIR)/libgcc_mock/libgcc_eh.a
```

Rust's `x86_64-pc-windows-gnu` staticlib references `libgcc_eh`, which the Rtools
UCRT toolchain doing R's final link does not provide. The mock satisfies the
reference. This is the UCRT-vs-MSVCRT mismatch from [`openssl.md`](openssl.md)
showing up as a concrete workaround, not a theoretical concern.

## Vendoring discipline

What gets vendored, and whether it has to, is the per-crate audit in
[`crates.md`](crates.md#avoiding-vendoring-open-task), now backed by a CI probe
of the Rtools45 prefix:

- Rtools **already ships** `libzstd` 1.5.7, `libdeflate` 1.25, `zlib` 1.3.1,
  `libjpeg` 9.6.0, `libbrotlienc` 1.2.0. So zstd (`zstd-sys`), libdeflate
  (`libdeflate-sys`), and zlib (`libz-sys`) vendoring duplicates an available
  Windows library.
- Rtools does **not** ship `duckdb`, `rocksdb`, or mozjpeg/`libjpeg-turbo`, so
  DuckDB, RocksDB, and mozjpeg genuinely must vendor.

## Method

```sh
# in a checkout of extendr-universe (`just setup` populates repos/)
grep -rhiE '^SystemRequirements:' repos/*/DESCRIPTION
grep -rhoE '^name = "[a-z0-9_-]+-sys"' repos/*/src/rust/Cargo.lock
grep -rnE '^PKG_LIBS' repos/*/src/Makevars*
```
