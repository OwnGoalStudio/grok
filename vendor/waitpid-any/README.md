# waitpid-any: Wait for any PID

[![crates.io](https://img.shields.io/crates/v/waitpid-any)](https://crates.io/crates/waitpid-any)
[![docs.rs](https://img.shields.io/docsrs/waitpid-any)](https://docs.rs/waitpid-any)
[![CI Status](https://github.com/oxalica/waitpid-any/actions/workflows/ci.yaml/badge.svg)](https://github.com/oxalica/waitpid-any/actions/workflows/ci.yaml)

[waitpid(2)] can only be used to wait for direct child processes, or it fails
immediately.

This crate provides a extension to wait for the exit of any process, not
necessarily child processes. Due to platform limitations, the exit reason and
status codes still cannot be retrieved.

[waitpid(2)]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/wait.html

## License

waitpid-any is distributed under the terms of either the MIT or the Apache 2.0
license, at your option. See [LICENSE-MIT](./LICENSE-MIT) and
[LICENSE-APACHE](./LICENSE-APACHE) for details.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be dual licensed as above, without any
additional terms or conditions.
