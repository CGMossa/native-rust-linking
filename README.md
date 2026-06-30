# native-rust-linking

Extracting a Rust crate's native link settings (using `openssl-sys`) so a
foreign build system such as R's can perform the final link.

See **[LINKING.md](LINKING.md)** for the full walkthrough: getting build
settings out of a `staticlib`, passing them to R via `Makevars`, the
per-platform link results (macOS / Ubuntu / Windows), and a system-vs-vendored
build benchmark.

The [`sys-deps/`](sys-deps/) registry records how Rtools and cargo each link a
dependency on Windows and where they diverge (CRT, provenance, build options),
indexed both by dependency ([`openssl.md`](sys-deps/openssl.md)) and by crate
([`crates.md`](sys-deps/crates.md)). A [corpus survey](sys-deps/corpus.md) of 52
extendr R packages backs it: only 2 link a real system library at all; the rest
are pure-Rust or vendor C/C++. Whether the vendored ones could avoid it (Rtools
shipping or being asked to add the lib) is an open task with its blockers listed.

MIT licensed.
