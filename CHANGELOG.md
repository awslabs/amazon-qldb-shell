# Changelog

## [2.0.0-alpha12] - [Unreleased]

### Added

- Added `\ping` which prints the round trip time.
- Added `\status` which prints the current ledger/region/version.

### Changed

- `--execute` is removed in favor of using unix-style pipes.
  - e.g. either `echo` or `cat` can be used to send PartiQL commands to the shell

## [2.0.0-alpha11] - 2021-05-18

### Fixed

- Copy-paste (CTRL-C + CTRL-V) should now work on Windows.

## [2.0.0-alpha10] - 2021-05-10

### Fixed

- Custom endpoints now work even if there is a trailing slash.
- Credentials are properly cached.
- Underlying driver updated to fix #72.
  - StartTransaction on a session with an open transaction would retry until the
    transaction timed out on the serer.

## [2.0.0-alpha9] - 2021-05-06

### Changed

- TLS is now provided by rustls.
  - This removes the need to install openssl libs on the end system.

## [2.0.0-alpha8] - 2021-05-06

### Added

- The prompt can now be customed with `ui.prompt` in the config file.
  - The following values can be interpolated:
    - $REGION: the name of the AWS region
    - $LEDGER: the name of the ledger
    - $ACTIVE_TRANSACTION: the string " \*" if a transaction is open

### Changed

- File logging now uses bunyan at maximum log level.
- The initial healthcheck client becomes the driver.
  - This means the initial HTTPS connection and credential loads can be reused.

## [2.0.0-alpha7] - 2021-05-05

### Added

- `\set input-mode [emacs|vi]` can now be used to toggle between Emacs/Vi keybindings
- Edit mode can be configured in the config file, e.g.:

  ```toml
  [ui]
  edit_mode = "Emacs" # or Vi
  ```

- Add `\set terminator-required [true|false]`
- Add `debug.log` in config files to enable logging to a file.
  - When this is set, the `-v` flag configures how much is logged. Nothing
    goes to stdout. This means you can use it to get lots of debugging
    without lots of stdout noise!

### Changed

- When running without a tty, the UI changes to be quieter
  - For example, the welcome message and CTRL-C or CTRL-D is suppressed
  - This is useful for `echo select * from foo|qldb --ledger example`
  - Query metrics are disabled by default
- Better error messages on connection fails

## [2.0.0-alpha6] - 2021-04-28

### Added

- Timestamps now render in `--format table`
- Byte arrays <= 32 bytes are rendered in `--format table`
- ALT/SHIFT + ENTER forces a newline
  - on Windows the sequence is SHIFT+ENTER
  - otherwise, ALT+ENTER (which would maximize the window on Windows)

### Changed

- User agent now includes both the driver and shell versions

## [2.0.0-alpha5] - 2021-04-23

### Changed

- Reworked verbose to be more useful.
  - rustyline is completely removed
  - at trace level, the pretty format is used

### Fixed

- Linux builds now use the older 18.04 (fixes #59)

## [2.0.0-alpha4] - 2021-04-23

### Added

- Added `--config PATH` to customize where we load config from.
- Improved table support.
  - `--format table` is now mostly complete
  - Includes support for nested content
  - Includes support for `select VALUE`
  - Timestamps are not yet supported

### Changed

- Started decoupling the program from CLI opts. This will allow better and dynamic configuration.
- `--verbose` can be used repeatedly to increase the level of logging.
  - We now use the `tracing` library instead of the `log` and `fern` libraries.
- Removed `--auto-commit` in favor of `--opt auto_commit=true`.
  - This is part of an overall strategy to avoid having a million flags.
- Added this file (CHANGELOG.md) to the release tar file.

### Fixed

- The version returned by `--version` or `-V` is now correct
  - Going forward, alpha versions are separated by a hyphen not a period

## [2.0.0.alpha3] - 2021-04-21

### Added

- Howbrew installation instructions
- Additional logging (when using `--verbose`)
- `--format table` as an option (this is not fully implemented)
- DESIGN.md as a vision document
- Connection issues (credentials, endpoints or invalid ledgers) fail faster
- Start to allow customization of the prompt

### Changed

- There are now three tiers of commands (PartiQL, special and commands):
  - The "special" commands are also commands but don't require a `\`.
  - Special commands include `help`, `quit`, `begin`, `commit`, etc.
- `\show-tables` is now `\show tables`

### Fixed

- Backtraces are on by default
- Any error prints a link to the bug tracker template (even if it's not a bug)

## [2.0.0.alpha2] - 2021-04-08

### Added

- Display query stats. The number of documents returned, read IOs usaged and timing information is now displayed for every query.
- Query stats can be disabled with `--no-query-metrics`
- Added `--terminator-required` which can be used to explicitly control sending commands to qldb (with `;`).
- Added abstractions around config and an environment. This will be used to make customization easy and DRY.
- Added `\show-tables`

### Changed

- `--auto-commit` now accepts `on|off`
- \*Shell is now built using async-await

### Fixed

- \*Don't quit on errors
- \*Allow some errors to leave a transaction open
- \*Improved help text

## [2.0.0.alpha1] - 2021-03-16

This is the first release of the 2.x series of the QLDB shell. The 1.x series was written in Python, while the 2.x series is written in Rust. We hope that you find the shell to be more
responsive and easier to install. The 2.x series will not see a stable release until we are confident in the quality of the library.

As of this milestone, the 2.x shell has feature parity with the 1.x shell.
