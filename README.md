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

### Configuration

After installation, the Shell will load default config file located at `$XDG_CONFIG_HOME/qldbshell/default_config.toml` during initialization. For example on OSX this will typically be at `/Users/username/Library/Application Support/qldbshell/default_config.toml`. You will need to create the `default_config.toml` config file manually after installation. The config file uses [TOML syntax](https://github.com/toml-lang/toml) and the format is as follows:

```
default_ledger = "my-ledger"

[ui]
# Determines whether each statement will be executed as a transaction or not. 
# By default this is on, meaning that statements are all executed as individual transactions.
auto_commit = [true|false] # default: true

# Set your prompt to your desired value. The following values can be interpolated:
#   - $REGION: the name of the AWS region
#   - $LEDGER: the name of the ledger
#   - $ACTIVE_TRANSACTION: the string " \*" if a transaction is open
prompt = your-prompt-syntax # default: "qldb$ACTIVE_TRANSACTION> "

# format = [ion|table]
#   ion: Prints the objects from the database as ION documents in text.
#   table: Tabulates the data and prints out the data as rows.
format = "table" # or ion default: ion

edit_mode = "Emacs" # or Vi default: Emacs

display_welcome = [true|false] # default: true

display_ctrl_signals = [true|false] # default: true

# Determines whether or not metrics will be emitted after the results of a query are shown.
display_query_metrics = [true|false] # default: true

# Set terminator_required to true indicates that pressing the enter key at the end of a line of input will not execute the command by itself. 
# Alternately, if you end your statement with a semi-colon (`;`) you will execute the statement.
terminator_required = [true|false] # default: false
```


An example minimal `default_config.toml` config file:

```
default_ledger = "my-ledger"

[ui]
# All uses default settings
```

### Building from source

See [HACKING.md](HACKING.md) for further instructions

## Command interface

### Database commands

* `start transaction` or `begin`
  * This starts a transaction.
* `commit`
  * This commits a transaction. If there is no transaction in progress, the shell reports an error saying that there is 
    no active transaction.
* `abort`
  * This aborts a transaction. If there is no transaction in progress, the shell reports an error saying that there is
    no active transaction. 
* `help`
  * Prints the lists of database and meta commands.
* `quit` or `exit`
  * Quits the shell.

### Shell Meta Commands

All commands to the shell itself will be prefixed with a backslash \\, e.g:

* `\use -l LEDGER_NAME [-p PROFILE] [-r REGION_CODE] [-s QLDB_SESSION_ENDPOINT]`
  * Switch to a different ledger (or: region, endpoint, AWS profile) without restarting the shell.
* `\set`
  * `\set edit-mode [emacs|vi]` Toggle between Emacs/Vi keybindings.
  * `\set terminator-required [true|false]` Toggle `terminator_required`.
* `\show tables`
  * Display a list of tables in the current ledger.
* `\status`
  * Prints out things like connection status, server ping latency, etc.
* `\env`
  * Prints out your current environment settings including where they were set from.
* `\ping`
  * Prints the round-trip time to the server.

## License

This project is licensed under the Apache-2.0 License.
