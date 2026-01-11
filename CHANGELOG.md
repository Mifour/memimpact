# Changelog

All notable changes to **MemImpact** will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/).

---
## [0.0.7] — 2026-01-11
### Refactor
- Refactor to have arguments into a simple struct and isolate arg parsing into a function
- add some tests about argument parsing
  
## [0.0.6] — 2025-12-21
### Improvements
- Refactored the parsing of proc/{}/stat and proc/{}/statm files
- Stronger validation and better error handling

## [0.0.5] — 2025-12-03
### Fixed
- Fixed a bug with children process that caused memimpact enter an infinite loop


## [0.0.4] — 2025-11-29
### Chore
- add aarch64 binary release on github

### Feature
- Add --output-file option to output in a designated file instead, stdout by default


## [0.0.3] — 2025-11-23
### Fixed
- Fixed the github release workflow


## [0.0.2] — 2025-11-23
### Fixed
- Added release GitHub Actions workflow (now active)

## [0.0.1] — 2025-11-23
### Added
- Initial release of **MemImpact**
- Basic command execution and memory tracking
- Measurement of peak RSS including child processes
- Linux `/proc` reader implementation
- Minimal CLI wrapper and example usage
