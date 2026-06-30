# crate -> system dependency

The registry indexed the other way: by the Rust crate that brings a native
dependency in, rather than by the dependency. For a Rust R package this is often
the more useful lookup, you read it off `Cargo.lock`, and it tells you whether
that crate needs an installed library, vendors its own C, or is pure Rust (and
so links nothing beyond the OS).

Data from the [corpus survey](corpus.md) (52 extendr packages). "Resolves by"
is what the crate does when built for an R package; "verdict" judges whether
that is the right call (see [Vendoring discipline](#vendoring-discipline)).

## Native C / C++ libraries

These compile or link an actual C/C++ library. Only these can need a
Rtools-supplied lib or duplicate one by vendoring.

"Status today" is current behaviour, not a fixed verdict. Whether each vendored
case could avoid vendoring is an open question, see
[Avoiding vendoring](#avoiding-vendoring-open-task).

| crate | system dep | resolves by | pure-Rust alt | packages | status today |
|---|---|---|---|---|---|
| `openssl-sys` | OpenSSL | system (`-lssl -lcrypto`); Rtools provides it | `rustls`+`ring` | arcgisplaces | system, not vendored |
| `libduckdb-sys` | DuckDB | vendored (`bundled`) | none | freestiler, ggsql | vendored; no Rtools lib |
| `oxrocksdb-sys` | RocksDB | vendored | none | roxigraph | vendored; no Rtools lib |
| `mozjpeg-sys` | mozjpeg | vendored | decode-only (`zune-jpeg`) | tinyimg | vendored; Rtools? (probe) |
| `libdeflate-sys` | libdeflate | vendored | `miniz_oxide` (slower) | tinyimg | vendored; Rtools? (probe) |
| `zstd-sys` | zstd (`libzstd`) | vendored | none (no prod encoder) | freestiler, gtfsrealtime, unsum | vendored; Rtools likely has it (probe) |
| `clang-sys` | libclang | build-time (bindgen) | n/a | roxigraph | build dep, not link dep |

The vendored rows are transitive in every case (DuckDB via the `duckdb` crate,
zstd via `pmtiles2`/`zip`/`parquet`, etc.), so the package author did not pick
to bundle C. Whether they *could* link a provided lib instead is the open task.

## Pure-Rust replacements (no system dep, no C)

The reason the corpus needs so few system libraries: the common C dependencies
have Rust reimplementations, and the packages use them. These link nothing
beyond the OS baseline (`ring` uses Windows CNG `bcrypt`, already in the
standard `Makevars.win` line).

| crate | replaces | packages |
|---|---|---|
| `rustls` + `ring` | OpenSSL / TLS | arcgisplaces, freestiler, ggsql, gtfsrealtime, pdfsigner |
| `zlib-rs` / `libz-rs-sys` | zlib (`libz-sys`) | freestiler, ggsql, gtfsrealtime, unsum |
| `miniz_oxide` | zlib/deflate (`flate2` backend) | freestiler, gtfsrealtime |
| `libbz2-rs-sys` | bzip2 (`bzip2-sys`) | gtfsrealtime |

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

**Per library:**

- **zstd** (`zstd-sys`): most actionable. Rtools probably ships `libzstd` (probe
  confirms) and the crate honours `ZSTD_SYS_USE_PKG_CONFIG`. Blockers: it is
  transitive across three packages with three different parents, the env var has
  to reach cargo through R's build, and the bundled build is cheap so the payoff
  is small.
- **libdeflate, mozjpeg** (`libdeflate-sys`, `mozjpeg-sys`): small, broadly
  useful C libs, so plausible Rtools-addition requests. Blockers: `.pc` files,
  crate system-link support, and the version/ABI coupling above.
- **DuckDB, RocksDB** (`libduckdb-sys`, `oxrocksdb-sys`): large C++ with their
  own toolchain needs. An Rtools addition is unlikely and vendoring is
  realistically the only option; the task here is just to confirm no system
  packaging exists, then accept it.
