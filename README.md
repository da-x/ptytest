# ptytest &emsp; [![Build Status]][travis] [![Latest Version]][crates.io] [![Docs badge]][Docs link] [![License badge]][License link]

[Build Status]: https://api.travis-ci.org/da-x/ptytest.svg?branch=master
[travis]: https://travis-ci.org/da-x/ptytest
[Latest Version]: https://img.shields.io/crates/v/ptytest.svg
[crates.io]: https://crates.io/crates/ptytest
[License badge]: https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg
[License link]: https://travis-ci.org/da-x/ptytest
[Docs badge]: https://docs.rs/ptytest/badge.svg
[Docs link]: https://docs.rs/ptytest

The `ptytest` crate provides a convenient way to test programs that write to
terminals, by matching over the state of the pseudo-terminal.

* Under [examples](examples/), see a simple tested program, and a test that uses `ptytest`'s API
in order to execute and test it.
* See [run-examples.sh](run-examples.sh) file, which builds the example program and runs the example test.

## License

`ptytest` is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `ptytest` by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
