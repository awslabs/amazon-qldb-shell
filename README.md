## QLDB Interactive Shell

## For Developers

### Installing Rust

Go to [Rustup](https://rustup.rs/) and follow the instructions to install rust. Then install `cmake`, for example on OSX:

```
brew install cmake
```

If you would like to modify the source code and work on the driver, after cloning the repo, simply run:

```
cargo test
```

To run the shell, use:

```
cargo run -- --ledger chess
```
```

And to run the test using the release optimized compilation, use:

cargo run --release -- --ledger chess
```

Note that the `"--"` indicates to Cargo that what follows are args for the shell.

## License

This project is licensed under the Apache-2.0 License.


[]: https://rustup.rs/]{Rustup}