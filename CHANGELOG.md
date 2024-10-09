# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.11] - 2024-10-09
### Details
#### Features
- Added parsing for ns:key by [@TheYoxy](https://github.com/TheYoxy)
- Using namespace separator provided inside the options by [@TheYoxy](https://github.com/TheYoxy)

## [0.2.10] - 2024-08-10
### Details
#### Miscellaneous Tasks
- Bumped version to 0.2.10 by [@TheYoxy](https://github.com/TheYoxy)

## [0.2.9] - 2024-08-10
### Details
#### Miscellaneous Tasks
- Bumped version to 0.2.9 by [@TheYoxy](https://github.com/TheYoxy)

## [0.2.8] - 2024-08-06
### Details
#### Miscellaneous Tasks
- Bumped version to 0.2.8 by [@TheYoxy](https://github.com/TheYoxy)

## [0.2.7] - 2024-08-06
### Details
#### Features
- Changed file_pattern to update cargo.toml files by [@TheYoxy](https://github.com/TheYoxy)

#### Miscellaneous Tasks
- Bumped version to 0.2.7 by [@TheYoxy](https://github.com/TheYoxy)

#### Performance
- Removed unnecessary clone to improve performance by [@TheYoxy](https://github.com/TheYoxy)

#### Styling
- Applied new rustfmt rules by [@TheYoxy](https://github.com/TheYoxy)

#### Testing
- Flatten test implementation by [@TheYoxy](https://github.com/TheYoxy)
- Replaced double quotes by single quotes in js code by [@TheYoxy](https://github.com/TheYoxy)
- Added test cases for visitor from i18next-parser by [@TheYoxy](https://github.com/TheYoxy)
- Added test cases for merge_hashes by [@TheYoxy](https://github.com/TheYoxy)

## [0.2.6] - 2024-07-25
### Details
#### Features
- Added more detailled time to generate by [@TheYoxy](https://github.com/TheYoxy)

#### Miscellaneous Tasks
- Bumped version to 0.2.6 by [@TheYoxy](https://github.com/TheYoxy)

## [0.2.5] - 2024-07-19
### Details
#### Bug Fixes
- Removed user panic on unsupported properties by [@TheYoxy](https://github.com/TheYoxy)
- Extracting correct namespace value from t function by [@TheYoxy](https://github.com/TheYoxy)

#### Features
- Added instrumentation feature for debugging by [@TheYoxy](https://github.com/TheYoxy)

#### Miscellaneous Tasks
- Bumped version to 0.2.5 by [@TheYoxy](https://github.com/TheYoxy)

## [0.2.4] - 2024-07-18
### Details
#### Miscellaneous Tasks
- Bumped version to 0.2.4 by [@TheYoxy](https://github.com/TheYoxy)

## [0.2.3] - 2024-07-17
### Details
#### Bug Fixes
- Corrected nix build by [@TheYoxy](https://github.com/TheYoxy)

#### Features
- Added more logs with colors by [@TheYoxy](https://github.com/TheYoxy)

#### Miscellaneous Tasks
- Bumped version to 0.2.3 by [@TheYoxy](https://github.com/TheYoxy)

#### Refactor
- Moved out bins to root folder by [@TheYoxy](https://github.com/TheYoxy)
- Splitted out crates into specific folders by [@TheYoxy](https://github.com/TheYoxy)
- Changed visibility of create methods by [@TheYoxy](https://github.com/TheYoxy)
- Created better scoped crates by [@TheYoxy](https://github.com/TheYoxy)

## [0.2.2] - 2024-07-16
### Details
#### Features
- Replaced serde_yaml with serde_yaml_ng by [@TheYoxy](https://github.com/TheYoxy)

#### Miscellaneous Tasks
- Bumped version to 0.2.2 by [@TheYoxy](https://github.com/TheYoxy)

#### Refactor
- Added more logs into generate_types method by [@TheYoxy](https://github.com/TheYoxy)

## [0.2.1] - 2024-07-16
### Details
#### Miscellaneous Tasks
- Bumped version to 0.2.1 by [@TheYoxy](https://github.com/TheYoxy)

## [0.2.0] - 2024-07-16
### Details
#### Bug Fixes
- Using current directory instead of executable directory as default by [@TheYoxy](https://github.com/TheYoxy)
- Extracting node doesnt always works by [@TheYoxy](https://github.com/TheYoxy)
- Object were resetted when handling plurals by [@TheYoxy](https://github.com/TheYoxy)
- Do not override values for non primary languages by [@TheYoxy](https://github.com/TheYoxy)

#### Documentation
- Added documentation for most of modules by [@TheYoxy](https://github.com/TheYoxy)
- Added better documentation in README.md by [@TheYoxy](https://github.com/TheYoxy)
- Added more documentation by [@TheYoxy](https://github.com/TheYoxy)

#### Features
- Improved global logging and QOL by [@TheYoxy](https://github.com/TheYoxy)
- Improved efficienty for parsing by [@TheYoxy](https://github.com/TheYoxy)
- Using multi_thread to analyze files by [@TheYoxy](https://github.com/TheYoxy)
- Added generation of a custom type options by [@TheYoxy](https://github.com/TheYoxy)
- Added base impl with intl_pluralrules crates by [@TheYoxy](https://github.com/TheYoxy)
- Imported intl_pluralrules crate into workspaces by [@TheYoxy](https://github.com/TheYoxy)
- Added options implementation in intl_pluralerules by [@TheYoxy](https://github.com/TheYoxy)
- Make cldr_parser compatible with version 45 by [@TheYoxy](https://github.com/TheYoxy)
- Using rust-toolchain by [@TheYoxy](https://github.com/TheYoxy)
- Checking that count value exists instead of getting its value by [@TheYoxy](https://github.com/TheYoxy)
- Added shell completion to nixos by [@TheYoxy](https://github.com/TheYoxy)
- Splitted flake definition by [@TheYoxy](https://github.com/TheYoxy)
- Added github actions by [@TheYoxy](https://github.com/TheYoxy)
- Using git-cliff to handle changelog by [@TheYoxy](https://github.com/TheYoxy)

#### Miscellaneous Tasks
- Using rust-overlay to build flake by [@TheYoxy](https://github.com/TheYoxy)
- Moving debug logs to trace level by [@TheYoxy](https://github.com/TheYoxy)
- Removed clippy issues by [@TheYoxy](https://github.com/TheYoxy)
- Bump oxc to latest version by [@TheYoxy](https://github.com/TheYoxy)
- Added cargo bump version on release by [@TheYoxy](https://github.com/TheYoxy)

#### Refactor
- Splitted file for better DX by [@TheYoxy](https://github.com/TheYoxy)

#### Styling
- Applied style configuration to all projects by [@TheYoxy](https://github.com/TheYoxy)

#### Testing
- Corrected code to pass the missing tests by [@TheYoxy](https://github.com/TheYoxy)
- Added tests for generate_types by [@TheYoxy](https://github.com/TheYoxy)
- Added test to check if the value is overriden in the merge_results when a value is provided in the string file by [@TheYoxy](https://github.com/TheYoxy)

## [0.1.0] - 2024-06-14
### Details
#### Bug Fixes
- Splitting entries correctly by [@TheYoxy](https://github.com/TheYoxy)

#### Features
- Init repo with ns extraction by [@TheYoxy](https://github.com/TheYoxy)
- Added jsx parsing by [@TheYoxy](https://github.com/TheYoxy)
- Applying better pattern to allow the app to be usable by [@TheYoxy](https://github.com/TheYoxy)
- Parsing count from code to enable plurialization by [@TheYoxy](https://github.com/TheYoxy)
- Added glob to filter folders by [@TheYoxy](https://github.com/TheYoxy)
- Reading all file in folder and configuration by [@TheYoxy](https://github.com/TheYoxy)
- Added better logging by [@TheYoxy](https://github.com/TheYoxy)
- Deploying app through flake by [@TheYoxy](https://github.com/TheYoxy)
- Added README.md by [@TheYoxy](https://github.com/TheYoxy)
- Added correct description for flake package by [@TheYoxy](https://github.com/TheYoxy)

#### Refactor
- Moved transformation method into transform mod by [@TheYoxy](https://github.com/TheYoxy)

#### Testing
- Added tempdir for testing by [@TheYoxy](https://github.com/TheYoxy)
- Added test to validate that the value isn't overriden when existing by [@TheYoxy](https://github.com/TheYoxy)

[0.2.11]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.2.10..0.2.11
[0.2.10]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.2.9..0.2.10
[0.2.9]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.2.8..0.2.9
[0.2.8]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.2.7..0.2.8
[0.2.7]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.2.6..0.2.7
[0.2.6]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.2.5..0.2.6
[0.2.5]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.2.4..0.2.5
[0.2.4]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.2.3..0.2.4
[0.2.3]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.2.2..0.2.3
[0.2.2]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.2.1..0.2.2
[0.2.1]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.2.0..0.2.1
[0.2.0]: https://github.com/TheYoxy/i18next-parser-rs/compare/0.1.0..0.2.0

<!-- generated by git-cliff -->
