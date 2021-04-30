# Changelog

## [2.0.0-alpha7] - [Unreleased]

### Added

   * `\set input-mode [emacs|vi]` can now be used to toggle between Emacs/Vi keybindings
   * Edit mode can be configured in the config file, e.g.:

     ```toml
     [ui]
     edit_mode = "Emacs" # or Vi
     ```

## [2.0.0-alpha6] - 2021-04-28

### Added

  * Timestamps now render in `--format table`
  * Byte arrays <= 32 bytes are rendered in `--format table`
  * ALT/SHIFT + ENTER forces a newline
    - on Windows the sequence is SHIFT+ENTER
    - otherwise, ALT+ENTER (which would maximize the window on Windows)
    
### Changed

  * User agent now includes both the driver and shell versions

## [2.0.0-alpha5] - 2021-04-23

### Changed

  * Reworked verbose to be more useful.
    - rustyline is completely removed
    - at trace level, the pretty format is used

### Fixed

   * Linux builds now use the older 18.04 (fixes #59)

## [2.0.0-alpha4] - 2021-04-23

### Added

  * Added `--config PATH` to customize where we load config from. 
  * Improved table support.
    - `--format table` is now mostly complete
    - Includes support for nested content
    - Includes support for `select VALUE`
    - Timestamps are not yet supported

### Changed

  * Started decoupling the program from CLI opts. This will allow better and dynamic configuration.
  * `--verbose` can be used repeatedly to increase the level of logging.
    - We now use the `tracing` library instead of the `log` and `fern` libraries.
  * Removed `--auto-commit` in favor of `--opt auto_commit=true`.
    - This is part of an overall strategy to avoid having a million flags.
  * Added this file (CHANGELOG.md) to the release tar file.
  
### Fixed

  * The version returned by `--version` or `-V` is now correct
    - Going forward, alpha versions are separated by a hyphen not a period

## [2.0.0.alpha3] - 2021-04-21

### Added

  * Howbrew installation instructions
  * Additional logging (when using `--verbose`)
  * `--format table` as an option (this is not fully implemented)
  * DESIGN.md as a vision document
  * Connection issues (credentials, endpoints or invalid ledgers) fail faster
  * Start to allow customization of the prompt 

### Changed

  * There are now three tiers of commands (PartiQL, special and commands):
    - The "special" commands are also commands but don't require a `\`.
    - Special commands include `help`, `quit`, `begin`, `commit`, etc.
  * `\show-tables` is now `\show tables`

### Fixed

  * Backtraces are on by default
  * Any error prints a link to the bug tracker template (even if it's not a bug)

## [2.0.0.alpha2] - 2021-04-08

### Added

  * Display query stats. The number of documents returned, read IOs usaged and timing information is now displayed for every query.
  * Query stats can be disabled with `--no-query-metrics`
  * Added `--terminator-required` which can be used to explicitly control sending commands to qldb (with `;`).
  * Added abstractions around config and an environment. This will be used to make customization easy and DRY.
  * Added `\show-tables`

### Changed

  * `--auto-commit` now accepts `on|off`
  * *Shell is now built using async-await

### Fixed

  * *Don't quit on errors
  * *Allow some errors to leave a transaction open
  * *Improved help text

## [2.0.0.alpha1] - 2021-03-16

This is the first release of the 2.x series of the QLDB shell. The 1.x series was written in Python, while the 2.x series is written in Rust. We hope that you find the shell to be more
responsive and easier to install. The 2.x series will not see a stable release until we are confident in the quality of the library.

As of this milestone, the 2.x shell has feature parity with the 1.x shell.
