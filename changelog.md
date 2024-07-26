# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com).

## [Unreleased]

### Added

- Support for tag manifests

### Changed

- Write a tag manifest when creating a bag
- Validate tag manifest if present when reading a bag

### Fixed

- Use absolute paths when reading payloads, should prevent from path traversal attacks

### Removed

- Trademark sign previously shown after the project description in version 
0.3.0

## [0.1.0] - 2024-07-21

### Added

- Read and validate bags, get paths of data payloads
- Create bags, add data payloads, finalize them
- Example: Create bag with BLAKE3 algorithm
- Example: Read zstd archive containing a tarball containing a bag 
- Tests, sample assets
- Checksum algorithm must be provided by crate consumer

