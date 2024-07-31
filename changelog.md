# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com).

## Unreleased

## 0.2.0 - 2024-08-01

### Added

- Support for tag manifests
- Added `Metadata` struct, read/write tags from/to file bag-data.txt
- Storing metadata tags inside `BagIt` struct
- Store file size in `Payload` struct

Supporting reading and writing commonly used tags through `Metadata` struct:
- `BagIt-Version`
- `Tag-File-Character-Encoding`
- `Bagging-Date` (with [`jiff`](http://docs.rs/jiff) crate)
- `Payload-Oxum`
- Custom tags with key/value stored as strings

### Changed

- Write a tag manifest when creating a bag
- Validate tag manifest if present when reading a bag
- Using new `Metadata` struct for reading and writing bagit.txt file

### Fixed

- Use absolute paths when reading payloads, should prevent from path traversal attacks

## 0.1.0 - 2024-07-21

### Added

- Read and validate bags, get paths of data payloads
- Create bags, add data payloads, finalize them
- Example: Create bag with BLAKE3 algorithm
- Example: Read zstd archive containing a tarball containing a bag 
- Tests, sample assets
- Checksum algorithm must be provided by crate consumer

