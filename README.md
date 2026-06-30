# native-rust-linking

Extracting a Rust crate's native link settings (using `openssl-sys`) so a
foreign build system such as R's can perform the final link.

See **[LINKING.md](LINKING.md)** for the full walkthrough: getting build
settings out of a `staticlib`, passing them to R via `Makevars`, the
per-platform link results (macOS / Ubuntu / Windows), and a system-vs-vendored
build benchmark.

The [`sys-deps/`](sys-deps/) registry records, per system dependency, how
Rtools and cargo each link it on Windows and where they diverge (CRT,
provenance, build options). First entry: [`sys-deps/openssl.md`](sys-deps/openssl.md).

MIT licensed.
