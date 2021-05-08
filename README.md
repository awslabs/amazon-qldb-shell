# QLDB Interactive Shell

[![License](https://img.shields.io/hexpm/l/plug.svg)](https://github.com/awslabs/amazon-qldb-shell/blob/main/LICENSE)
[![CI Build](https://github.com/awslabs/amazon-qldb-shell/workflows/CI%20Build/badge.svg)](https://github.com/awslabs/amazon-qldb-shell/actions?query=workflow%3A%22CI+Build%22)


## Installation

[**v2.0.0.alpha9** is now available!](https://github.com/awslabs/amazon-qldb-shell/releases/tag/v2.0.0-alpha9)

In general, see [releases][releases] for the latest and greatest. We provide
prebuilt binaries for Linux, Windows and macOS. Find your platform, download,
extract and run!

If your platform is not supported, feel free to open an issue!

[releases]: https://github.com/awslabs/amazon-qldb-shell/releases

### macOS

On macOS, we integrate with the `aws/tap` Homebrew tap:

   ```
   xcode-select --install # required to use Homebrew
   brew tap aws/tap # Add AWS as a Homebrew tap
   brew install qldbshell
   qldb --ledger <your-ledger>
   ```

### Building from source

See [HACKING.md][HACKING.md] for further instructions

## License

This project is licensed under the Apache-2.0 License.
