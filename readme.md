# async_bagit

![CI status](https://github.com/deadbaed/async-bagit-rs/actions/workflows/rust.yml/badge.svg)
[![crates.io version](https://img.shields.io/crates/v/async_bagit)](https://crates.io/crates/async_bagit)
[![docs.rs](https://img.shields.io/docsrs/async_bagit)](https://docs.rs/async_bagit)

Rust library to create and read [BagIt](https://en.wikipedia.org/wiki/BagIt) containers.

## Requirements to use in your crate

- Tokio runtime
- Nightly Rust until features shown in table below are stabilized, sorry. Feel free to propose a patch to have something working on stable in the meantime!

| Nightly feature | Tracking issue |
| --- | --- |
| `iter_next_chunk` | [#98326](https://github.com/rust-lang/rust/issues/98326) |

## Notes when using the crate

### Checksum algorithms

The [BagIt spec](https://datatracker.ietf.org/doc/html/rfc8493) wants checksums on payload content inside bags.
While this crate handles checksums and can compute checksums on files, this crate **does not include** any checksum algorithms. I do not want to force crate consumers to use checksum x or y, you will have to provide the algorithm.

Any struct implementing the `Digest` trait from the [`digest`](https://docs.rs/digest) crate will be accepted for checksums computation. Most algorithms in the Rust ecosystem implement this trait already.
I am not an expert, but as of this writing (July 2024) I would recommend using [BLAKE3](https://docs.rs/blake3), [BLAKE2](https://docs.rs/blake2) or [SHA512](https://docs.rs/sha2), in that order.

### No special treatment of files

For opening bags, the crate reads only directories on the file system containing BagIt bags, compression or encryption is out of the scope, you will need to handle that yourself.

When creating bags, the crate **will copy files** when adding them to the bag. Make sure you have enough storage space and writing permission when creating your bags.

## RFC limitations and quirks

- UTF-8 only. This is a Rust crate, everything is UTF-8.
- No multi checksums, only one at a time.
- The RFC says in section 2.4 that `bag creation and validation tools MUST support the SHA-256 and SHA-512 algorithms`. This crate requires you to bring your own algorithm for checksums, so I guess this crate will never be RFC compliant?
- Checksums encoding are only supported in **lowercase**.
- This crate has been written for and on a UNIX system, so Windows file paths were not considered at all. Furthermore, casing in filenames was also not considered as well. Feel free to improve this if this is an issue for you.

## TODO

- [ ] `bag-info.txt` with common labels
- [ ] `fetch.txt`: I do not have any use for this yet, and seems a bunch of work to implement it
- [ ] Support multiple checksum algorithms at the same time
- [ ] Respect the spec regarding filename casing

I do not expect this crate to be fully compliant with the RFC or with other tools handling BagIt bags (although I am not against it, on the contrary).

My usage of BagIt will be creating bags with this crate, and opening them with this crate as well.

## Acknowledgments

If you are looking for a sync Rust implementation of BagIt https://github.com/pwinckles/bagr is a nice one!
