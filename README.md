# native-rust-linking

Extracting a Rust crate's native link settings (using `openssl-sys`) so a
foreign build system such as R's can perform the final link.

See **[LINKING.md](LINKING.md)** for the full walkthrough: getting build
settings out of a `staticlib`, passing them to R via `Makevars`, the
per-platform link results (macOS / Ubuntu / Windows), and a system-vs-vendored
build benchmark.

MIT licensed.
