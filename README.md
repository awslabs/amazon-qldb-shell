# QLDB Interactive Shell

[![License](https://img.shields.io/hexpm/l/plug.svg)](https://github.com/awslabs/amazon-qldb-shell/blob/main/LICENSE)
[![CI Build](https://github.com/awslabs/amazon-qldb-shell/workflows/CI%20Build/badge.svg)](https://github.com/awslabs/amazon-qldb-shell/actions?query=workflow%3A%22CI+Build%22)

## Welcome to the v2.0 branch!

Hi traveler, you've found your way to **alpha quality software**. Here be
dragons.

The QLDB Shell is undergoing a complete rewrite in Rust to make it faster and
lower latency with zero external dependencies. This is an early alpha release
which is not intended for use in production systems. For the time being the
Python-based shell on the master branch continues to be the production-ready
release. Please contact us if you experiment with this early alpha release and
have feedback you'd like to share with us.

If you'd like to follow along, [CHANGELOG.md](CHANGELOG.md) is kept up to date
with each commit.

## Installation

[**v2.0.0.alpha13** is now
available!](https://github.com/awslabs/amazon-qldb-shell/releases/tag/v2.0.0-alpha13)

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

After installation, the Shell will load default config file located at `$XDG_CONFIG_HOME/qldbshell/default_config.toml` during initialization. For example on OSX this will typically be at `/Users/username/Library/Application Support/qldbshell/default_config.toml`. You will need to create the `default_config.toml` config file manually after installation. An example minimal `default_config.toml` config file:
```
default_ledger = "my-ledger"

[ui]
edit_mode = "Vi"
```

### Building from source

See [HACKING.md](HACKING.md) for further instructions

## License

This project is licensed under the Apache-2.0 License.
