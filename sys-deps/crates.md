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

| crate | system dep | resolves by | pure-Rust alt | packages | verdict |
|---|---|---|---|---|---|
| `openssl-sys` | OpenSSL | system (`-lssl -lcrypto`); Rtools provides it | `rustls`+`ring` | arcgisplaces | OK (system, not vendored) |
| `libduckdb-sys` | DuckDB | vendored (`bundled`) | none | freestiler, ggsql | necessary |
| `oxrocksdb-sys` | RocksDB | vendored | none | roxigraph | necessary |
| `mozjpeg-sys` | mozjpeg | vendored | decode-only (`zune-jpeg`) | tinyimg | necessary (encoder) |
| `libdeflate-sys` | libdeflate | vendored | `miniz_oxide` (slower) | tinyimg | deliberate (speed) |
| `zstd-sys` | zstd (`libzstd`) | vendored | none (no prod encoder) | freestiler, gtfsrealtime, unsum | redundant but transitive |
| `clang-sys` | libclang | build-time (bindgen) | n/a | roxigraph | build dep, not link dep |

`zstd-sys` is the only one that vendors a library the platform usually already
has (Rtools45 ships `libzstd`). It is pulled transitively (`pmtiles2`, `zip`,
`parquet`), not chosen, and the only way off the bundled copy is
`ZSTD_SYS_USE_PKG_CONFIG`, which is fragile cross-platform. So: redundant on
paper, not worth changing.

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

## Vendoring discipline

Across the corpus, vendoring tracks need, with one transitive exception:

- **Vendored because there is no alternative on CRAN's build farm:** DuckDB,
  RocksDB, mozjpeg, libdeflate. Bundling C source is the only option; the
  resulting `.a` is self-contained and needs no installed lib.
- **Not vendored because a Rust-native version exists, and the packages use
  it:** zlib (`flate2`/`zlib-rs`), bzip2 (`libbz2-rs-sys`), TLS (`rustls`+`ring`).
  No package pulls `libz-sys`, `bzip2-sys`, or `openssl-src`. The one OpenSSL
  user links the system lib instead of vendoring.
- **Vendored despite the platform having it:** only `zstd-sys`, and only
  transitively (file-format crates), with no practical pure-Rust escape.

So no package vendors unnecessarily in any avoidable way. The libraries that get
bundled have to be; the ones that could be system or pure-Rust already are.
