# audit: GitHub-only (non-CRAN) extendr packages

The 12 extendr R packages added to the corpus from GitHub search (not on CRAN),
checked for system-dependency, linking, and vendoring hygiene. They are visibly
less disciplined than the CRAN set (see [`corpus.md`](corpus.md)); this lists
what each could fix. Neutral observations against common practice, not a verdict
on the packages.

Baseline for reference (the rextendr `Makevars.win` standard, 53/64 of the
corpus): `-lws2_32 -ladvapi32 -luserenv -lbcrypt -lntdll`.

## Findings (most to least significant)

1. **`masreml` — links/builds OpenBLAS instead of R's BLAS.** Uses
   `ndarray-linalg` with `openblas-system` (Unix) and `openblas-static`
   (Windows, which compiles OpenBLAS from source). R already provides
   BLAS/LAPACK (`Rblas`/`Rlapack`); linking a second BLAS risks duplicate
   symbols and OpenMP/threading conflicts with R's, and the static build is
   heavy. It also vendors OpenSSL (`openssl-src`) though Rtools and most systems
   provide it. `SystemRequirements` (`Cargo, rustc >= 1.70.0`) declares neither
   the BLAS nor the OpenSSL build. Fix: link R's BLAS/LAPACK; use system OpenSSL;
   declare what is needed.

2. **`extendrSVGdevice`, `recolysis` — no `SystemRequirements` at all.** An
   extendr package should declare at least `Cargo (Rust's package manager),
   rustc` so the build system and CRAN know Rust is required. Both omit the field
   entirely.

3. **`hftokenizers` — incomplete Windows link line, no CRT mock.** `Makevars.win`
   is `-lws2_32 -ladvapi32 -luserenv` (missing `-lbcrypt -lntdll`) and lacks the
   `libgcc_eh` mock. Fragile: if the Rust side pulls `getrandom`/`ring` (CNG
   `bcrypt`) or std symbols in `ntdll`, the link breaks. (It does correctly
   declare `libclang`/`llvm-config`.)

4. **`zr` — C `libz-sys` + `zstd-sys`, both shipped by Rtools.** Pulls zlib and
   zstd from C while Rtools provides both (zlib 1.3.1, libzstd 1.5.7), and the
   tree already contains pure-Rust `zlib-rs`. Avoidable C dependency / internal
   inconsistency.

5. **`oxbow` — vendors `libdeflate-sys` + `bzip2-sys` (C) alongside pure-Rust
   `libbz2-rs-sys`.** libdeflate is shipped by Rtools (1.25); bzip2 is pulled
   twice (C `bzip2-sys` and Rust `libbz2-rs-sys`). `Makevars.win` is also missing
   `-lntdll`.

6. **`vctrsrs` — malformed `SystemRequirements`** (`Cargo (rustc package
   manager)`): garbled, and omits `rustc`.

Minor: `extendrSVGdevice` and `oxbow` `Makevars.win` omit `-lntdll`.

## Clean / good practice

- **`vdjmatchR`** computes `Makevars.win` `PKG_LIBS` at build time via
  `rextendr:::rustc_link_flags()` rather than hardcoding the baseline. This is the
  most robust approach: the link line tracks whatever the toolchain actually
  needs, no manual drift.
- **`osmnxr`, `rpic`, `gepafer`, `tynding`** — standard baseline, `libgcc_eh`
  mock present, proper `SystemRequirements`. Pure Rust, nothing to flag.

## Pattern

The recurring issues are all symptoms of no CRAN gate: undeclared system
dependencies (`masreml`, `extendrSVGdevice`, `recolysis`), C `-sys` crates where
the CRAN set uses pure-Rust or system libs (`zr`, `oxbow`, `masreml`), and
hand-written `Makevars.win` lines that drift from the standard baseline
(`hftokenizers`, `oxbow`, `extendrSVGdevice`). `vdjmatchR`'s dynamic link line is
the structural fix for the last one.
