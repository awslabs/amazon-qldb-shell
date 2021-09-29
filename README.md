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

## Prerequisites

This readme assumes you have an AWS account, an environment configured with AWS credentials, as well as IAM users/roles with appropriate access to QLDB. This readme focuses on shell configuration and commands. For a guide about prerequisites and general use of the shell, please see the [Developer Guide: Using the Amazon QLDB Shell](https://docs.aws.amazon.com/qldb/latest/developerguide/data-shell.html)

## QLDB Core Concepts

- QLDB ensures that transactions are consistent upon commit by using [optimistic currency control (OCC)](https://docs.aws.amazon.com/qldb/latest/developerguide/concurrency.html#concurrency.occ).
- In QLDB, every statement (including every SELECT query) must run in a transaction.
- By default, the shell enables auto-commit mode. In this mode, the shell interprets each command that you enter as a separate PartiQL statement, meaning that you don't have to run `start transaction` and `commit` manually each time. This is configurable (see below).
- An interactive transaction adheres to QLDB's [transaction timeout limit](https://docs.aws.amazon.com/qldb/latest/developerguide/limits.html#limits.fixed). If you don't commit a transaction within 30 seconds of starting it, QLDB automatically expires the transaction and rejects any changes made during the transaction. Then, the shell displays an error message and returns to the normal command prompt. To retry, you must enter the begin or start transaction command again to begin a new transaction.
  - Consider reading more in our guide for [optimizing query performance](https://docs.aws.amazon.com/qldb/latest/developerguide/working.optimize.html).
- QLDB supports a _subset_ of the PartiQL query language. When you use the QLDB shell to query data in Amazon QLDB, you write statements in PartiQL, but results are shown in Amazon Ion (this is configurable). PartiQL is intended to be SQL-compatible, whereas Ion is an extension of JSON. This leads to syntactic differences with how you notate data in your queries, compared to how the QLDB console presents your query results. Further details are available in the [Developer Guide: Querying Ion with PartiQL](https://docs.aws.amazon.com/qldb/latest/developerguide/ql-reference.query.html).
- This QLDB shell is used for the data plane only. To interact with the control plane, use the [AWS CLI](https://docs.aws.amazon.com/qldb/latest/developerguide/Tools.CLI.html)

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

After installation, the Shell will load default config file located at
`$XDG_CONFIG_HOME/qldbshell/config.ion` during initialization. On Linux and
Macos, this will typically be at `~/.config/qldbshell/config.ion`. If such a
file doesn't exist, the shell will run with default settings.

You can create a `config.ion` config file manually after installation. The
config file uses [Ion][ion]. If you're new to Ion, you can consider the file to
be JSON with support for comments and you'll be just fine!

```ion
{
  default_ledger: "my-ledger",

  ui: {
    // Determines whether each statement will be executed as a transaction or not.
    // By default this is on, meaning that statements are all executed as individual transactions.
    auto_commit: true, // the default; can be set to false

    // Set your prompt to your desired value. The following values can be interpolated:
    //   - $REGION: the name of the AWS region
    //   - $LEDGER: the name of the ledger
    //   - $ACTIVE_TRANSACTION: the string " \*" if a transaction is open
    prompt: "your-prompt-syntax", // default: "qldb$ACTIVE_TRANSACTION> "

    // format = [ion|table]
    //   ion: Prints the objects from the database as ION documents in text.
    //   table: Tabulates the data and prints out the data as rows.
    format: "table", // or ion default: ion

    edit_mode: "Emacs", // or Vi default: Emacs

    // Can be toggled to suppress some messaging when runnning in interactive mode
    display_welcome: true, // the default; can be set to false
    display_ctrl_signals: true,

    // Determines whether or not metrics will be emitted after the results of a query are shown.
    display_query_metrics: true,

    // Set terminator_required to true indicates that pressing the enter key at the end of a line of input will not execute the command by itself.
    // Alternately, if you end your statement with a semi-colon (`;`) you will execute the statement.
    terminator_required: true
  }
}
```

An example minimal `config.ion` config file:

```ion
{
  default_ledger: "my-ledger"
}
```

If `default_ledger` is not set, then `--ledger` becomes a required CLI parameter.

[ion]: https://amzn.github.io/ion-docs/

### Building from source

See [HACKING.md](HACKING.md) for further instructions

## Command interface

### Shell Keys

- Enter
  - Runs the statement
- Escape+Enter (macOS, \*nix) or Shift+Enter (Windows)
  - Starts a new line to enter a statement that spans multiple lines. You can also copy input text with multiple lines and paste it into the shell. For instructions on setting up Option instead of Escape as a Meta key in macOS, see the [OS X Daily](https://osxdaily.com/2013/02/01/use-option-as-meta-key-in-mac-os-x-terminal/) site.
- Ctrl+C
  - Cancels the current command.
- Ctrl+D
  - EOF / exits the current level of the shell. If not in a transaction, exits the shell. If in a transaction, aborts the transaction.

### Database commands

- `start transaction` or `begin`
  - Manually starts a transaction. You can run multiple statements within a transaction interactively, or non-interactively by batching commands and statements sequentially. Transactions that are not committed within 30 seconds will time out, and QLDB will reject any changes made during the transaction. For more details and examples, see the [QLDB Developer Guide](https://docs.aws.amazon.com/qldb/latest/developerguide/data-shell.html#data-shell-transactions).
- `commit`
  - Commits a transaction. If there is no transaction in progress, the shell reports an error saying that there is no active transaction.
- `abort`
  - Aborts a transaction. If there is no transaction in progress, the shell reports an error saying that there is no active transaction.
- `help`
  - Prints the lists of database and meta commands.
- `quit` or `exit`
  - Quits the shell.

### Shell Meta Commands

All commands to the shell itself will be prefixed with a backslash \\, e.g:

- `\use -l LEDGER_NAME [-p PROFILE] [-r REGION_CODE] [-s QLDB_SESSION_ENDPOINT]`
  - Switch to a different ledger (or: region, endpoint, AWS profile) without restarting the shell.
- `\set`
  - `\set edit-mode [emacs|vi]` Toggle between Emacs/Vi keybindings.
  - `\set terminator-required [true|false]` Toggle `terminator_required`.
- `\show tables`
  - Display a list of active tables in the current ledger.
- `\status`
  - Prints out your current region, ledger and Shell version.
- `\env`
  - Prints out your current environment settings including where they were set from.

## License

This project is licensed under the Apache-2.0 License.
