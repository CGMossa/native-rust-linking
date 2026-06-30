# crate -> system dependency

The registry indexed the other way: by the Rust crate that brings a native
dependency in, rather than by the dependency. For a Rust R package this is often
the more useful lookup, you read it off `Cargo.lock`, and it tells you whether
that crate needs an installed library, vendors its own C, or is pure Rust (and
so links nothing beyond the OS).

Data from the [corpus survey](corpus.md) (64 extendr packages). The last column
records whether Rtools45 already ships the library, from the CI probe
([`../results/rtools-available-libs.txt`](../results/rtools-available-libs.txt)):
if it does, the `-sys` crate could link it instead of bundling C (the
[avoiding-vendoring](#avoiding-vendoring-open-task) task).

## Native C / C++ libraries

These compile or link an actual C/C++ library. Only these can need a
Rtools-supplied lib or duplicate one by vendoring.

| crate | system dep | resolves by | pure-Rust alt | packages | Rtools45 ships it? |
|---|---|---|---|---|---|
| `openssl-sys` | OpenSSL | arcgisplaces: system (`-lssl -lcrypto`); masreml: vendored (`openssl-src`) | `rustls`+`ring` | arcgisplaces, masreml | yes (3.6.0) |
| `openblas-src` | OpenBLAS / LAPACK | linked (`openblas-static`/`-system`) | none | masreml | R itself provides BLAS/LAPACK |
| `zstd-sys` | zstd | vendored | none (no prod encoder) | freestiler, gtfsrealtime, unsum, zr | **yes (libzstd 1.5.7)** |
| `libdeflate-sys` | libdeflate | vendored | `miniz_oxide` (slower) | oxbow, tinyimg | **yes (1.25)** |
| `libz-sys` | zlib | links/builds zlib | `zlib-rs` (pure Rust) | zr | **yes (zlib 1.3.1)** |
| `bzip2-sys` | bzip2 | vendored | `libbz2-rs-sys` (pure Rust) | oxbow | not probed |
| `libduckdb-sys` | DuckDB | vendored (`bundled`) | none | freestiler, ggsql | no |
| `oxrocksdb-sys` | RocksDB | vendored | none | roxigraph | no |
| `mozjpeg-sys` | mozjpeg | vendored | decode-only (`zune-jpeg`) | tinyimg | no (libjpeg 9.6.0, not mozjpeg) |
| `clang-sys` | libclang | build-time (bindgen) | n/a | roxigraph, hftokenizers | build dep, not link dep |

The vendored rows are transitive in every case (DuckDB via the `duckdb` crate,
zstd via `pmtiles2`/`zip`/`parquet`, libdeflate/bzip2 via `oxbow`'s readers), so
the package author did not pick to bundle C. The probe shows the top four
(`zstd`, `libdeflate`, `zlib`, plus brotli) are already in Rtools, so on Windows
those bundled copies duplicate an available library; whether a package *should*
link them instead is the open task below.

## Pure-Rust replacements (no system dep, no C)

The reason the corpus needs so few system libraries: the common C dependencies
have Rust reimplementations, and the packages use them. These link nothing
beyond the OS baseline (`ring` uses Windows CNG `bcrypt`, already in the
standard `Makevars.win` line).

| crate | replaces | packages |
|---|---|---|
| `rustls` + `ring` | OpenSSL / TLS | arcgisplaces, freestiler, ggsql, gtfsrealtime, oxbow, pdfsigner |
| `zlib-rs` / `libz-rs-sys` | zlib (`libz-sys`) | freestiler, ggsql, gtfsrealtime, oxbow, tynding, unsum |
| `miniz_oxide` | zlib/deflate (`flate2` backend) | 13 packages |
| `libbz2-rs-sys` | bzip2 (`bzip2-sys`) | gtfsrealtime, oxbow |

## OS-API binding `-sys` crates (nothing to install)

These have `-sys` in the name but bind always-present OS libraries, not
installable packages. They appear constantly and need no system dependency.

| crate | binds | links |
|---|---|---|
| `windows-sys` | Win32 API | `kernel32`, `ws2_32`, `advapi32`, `bcrypt`, `ntdll`, `userenv`, ... |
| `core-foundation-sys`, `security-framework-sys`, `system-configuration-sys` | macOS frameworks | `CoreFoundation`, `Security`, ... |
| `linux-raw-sys` | Linux syscalls | none (constants) |
| `js-sys`, `web-sys` | wasm / JS | none (wasm target) |

## Avoiding vendoring (open task)

The "status today" verdicts are not fixed. Vendoring is sometimes avoidable, but
the routes are not obvious, so this is an exploration to run per library, not a
settled call.

**Routes off vendoring:**

1. **Rtools already ships the lib.** Then the `-sys` crate can `pkg-config` and
   link it instead of bundling C. Rtools' static-lib set is large; the CI probes
   it (`results/rtools-available-libs.txt`).
2. **Ask Rtools to add it.** Rtools' library set is curated and additive (the
   changelog shows libs appearing, e.g. `nghttp2` @6536). A C lib several
   packages share is a candidate to request as a packaged Rtools static lib,
   after which they all link it and none vendors.
3. **System lib on Unix/macOS** via `pkg-config` + a `SystemRequirements` line,
   the way `arcgisplaces` does for OpenSSL.
4. **A pure-Rust replacement matured** (track `ruzstd` for zstd decode, etc.).
5. **Drop the transitive puller** or feature-gate the codec off upstream.

**Challenges (why it is not obvious):**

1. **CRAN portability.** A system dependency must exist on every platform CRAN
   builds (Windows, macOS, many Linux). Vendoring is the safe default precisely
   because it removes that variability; a `SystemRequirements` line shifts the
   burden onto users and CRAN's machines.
2. **CRT / ABI match (Windows).** A Rtools static lib is UCRT, built with the
   exact GCC/MinGW Rtools ships; a Rust `-gnu` staticlib is MSVCRT. A provided
   lib has to link cleanly into R's final link alongside the Rust archive (the
   same mismatch that forces the `libgcc_eh` mock in `arcgisplaces`/`pdfsigner`).
3. **The `-sys` crate must support system linking.** It needs a documented
   opt-out of vendoring (`ZSTD_SYS_USE_PKG_CONFIG`, a `*_NO_VENDOR` env var, a
   `pkg-config` feature). If it hardcodes bundling, you patch or fork it.
4. **pkg-config and `.pc` discoverability.** System linking relies on a `.pc`
   file existing and being found, on each platform. Not every Rtools lib ships
   one; not every build environment has `pkg-config` wired up.
5. **Version coupling.** A `-sys` crate's bindings target a specific lib version.
   Whatever Rtools or the OS ships may be older or newer than the crate expects,
   breaking the build or needing a version gate. Vendoring pins a known-good
   version.
6. **Transitive control.** The C lib is pulled by an upstream crate
   (`pmtiles2`/`zip`/`parquet`), not declared directly. Removing it needs
   feature flags the upstream may not expose, a different upstream, or a fork.
7. **Pure-Rust gaps.** Some have no production Rust equivalent (zstd encoding,
   mozjpeg-quality JPEG). Switching loses functionality or quality.
8. **Requesting an Rtools addition is slow and Windows-only.** It is a maintainer
   request bound to the Rtools/CRAN release cycle, may be declined (especially
   large C++ like DuckDB/RocksDB), and even if accepted only fixes Windows, not
   macOS or Linux.
9. **Double-linking risk.** If R already links the same library elsewhere,
   linking it again (rather than vendoring into a private archive) can clash on
   symbols at the final link.
10. **Reproducibility.** Vendored source builds identically everywhere and over
    time; depending on whatever the system ships reintroduces drift.

**Per library** (the CI probe settles route 1: Rtools45 ships `libzstd` 1.5.7,
`libdeflate` 1.25, `zlib` 1.3.1, `libjpeg` 9.6.0, `libbrotlienc` 1.2.0; it does
not ship `duckdb`, `rocksdb`, or mozjpeg/`libjpeg-turbo`):

- **zstd, libdeflate, zlib** (`zstd-sys`, `libdeflate-sys`, `libz-sys`): Rtools
  ships all three, so the link route exists on Windows. Most actionable. Blockers:
  each is transitive (zstd via `pmtiles2`/`zip`/`parquet`; libdeflate/zlib via
  `oxbow`), the crate's vendoring opt-out (`ZSTD_SYS_USE_PKG_CONFIG`, etc.) has to
  reach cargo through R's build, and macOS/Linux still need the lib present. The
  bundled build is cheap, so the payoff is mostly avoiding a duplicate.
- **OpenBLAS / LAPACK** (`masreml`, via `ndarray-linalg`): the clearest avoidable
  case. R ships its own BLAS/LAPACK (`Rblas`/`Rlapack`), so a package should link
  R's rather than `openblas-static`. Blocker: `ndarray-linalg` needs a backend
  feature wired to R's BLAS, and ABI/threading has to match R's build.
- **OpenSSL** (`masreml`, vendored via `openssl-src`): Rtools ships OpenSSL 3.6.0,
  so `masreml` could link the system lib as `arcgisplaces` does, rather than
  vendoring. Same routes as [`openssl.md`](openssl.md).
- **mozjpeg** (`mozjpeg-sys`): Rtools ships `libjpeg` but not mozjpeg (a distinct
  fork), so a link route needs an Rtools-addition request; vendoring stays for now.
- **DuckDB, RocksDB** (`libduckdb-sys`, `oxrocksdb-sys`): large C++ with their own
  toolchain needs, absent from Rtools. An addition is unlikely and vendoring is
  realistically the only option; confirm no system packaging exists, then accept it.
