# openssl

Source of the numbers below: the `windows-linking` CI run (commit `a03eb10`),
files [`../results/rtools-pkgconfig-openssl.txt`](../results/rtools-pkgconfig-openssl.txt)
and [`../results/x86_64-pc-windows-gnu-native-static-libs.txt`](../results/x86_64-pc-windows-gnu-native-static-libs.txt).

## Rtools (R's build system)

`pkg-config --libs --static openssl`, OpenSSL 3.6.0, from
`C:/rtools45/x86_64-w64-mingw32.static.posix`. This matches CRAN's Rtools45
build 6768 (2026-02-04), which ships OpenSSL 3.6.0
([news](https://cran.r-project.org/bin/windows/Rtools/rtools45/news.html);
progression 3.4.0 @6536 → 3.5.0 @6691 → 3.6.0 @6768):

```
-lssl -lcrypto -lz -lws2_32 -lgdi32 -lcrypt32
```

## cargo

`--print native-static-libs`, `x86_64-pc-windows-gnu`, vendored OpenSSL 3.6.3:

```
-lgdi32 -luser32 -lcrypt32 -lws2_32 -ladvapi32 -lkernel32 -lntdll -luserenv -lws2_32 -ldbghelp
```

## Side by side

| | Rtools `pkg-config` | cargo `native-static-libs` |
|---|---|---|
| provider | Rtools45 static OpenSSL | `openssl-sys` vendored (`openssl-src`) |
| OpenSSL version | 3.6.0 | 3.6.3 |
| C runtime | UCRT | MSVCRT (Rust `-gnu` target) |
| `ssl` / `crypto` | `-lssl -lcrypto` (explicit) | bundled into the `.a` (static) |
| OpenSSL's Win32 deps | `crypt32 ws2_32 gdi32` | `crypt32 ws2_32 gdi32` **+ `advapi32 user32`** |
| zlib | `-lz` (built with zlib) | none (vendored is `no-zlib`) |
| Rust std baseline | n/a | `kernel32 ntdll userenv dbghelp` |

## Agree / diverge

- **Agree:** OpenSSL's core Win32 dependencies — `crypt32`, `ws2_32`, `gdi32`.
- **Diverge:**
  - **zlib** — Rtools' OpenSSL pulls `-lz`; vendored does not.
  - **`ssl`/`crypto` provenance** — explicit libs from the Rtools prefix vs
    statically bundled into the Rust archive.
  - **`advapi32`/`user32`** — cargo lists them; Rtools' `.pc` does not declare
    them (they still resolve at R's final link).
  - **std baseline** — cargo's note includes the Rust runtime libs; a
    `pkg-config` answer describes only OpenSSL.
  - **C runtime** — invisible in the lib names, but UCRT vs MSVCRT is the real
    ABI fault line for mixing a Rust `-gnu` staticlib into an R/Rtools package.
  - **version** — 3.6.0 vs 3.6.3.

## For an R package

R links the package, and supplies OpenSSL itself via Rtools `pkg-config`, so
`Makevars.win` `PKG_LIBS` already carries `-lssl -lcrypto` and their deps. The
Rust side should therefore **not vendor** OpenSSL (avoid linking it twice) and
only needs to contribute its std baseline. Match the CRT: build Rust for the
toolchain R uses.
