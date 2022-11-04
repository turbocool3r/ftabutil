# ftabutil

A simple utility to build and unpack 'ftab' (aka 'rkosftab') images found in firmware images of accessories produced by Apple.

## A quick into 'ftab' file format

'ftab' (which probably means **F**ile **TAB**le) files are simple tables of files where a key is a 4-byte tag (e.g. 'rkos' which stands for **R**T**K**it **OS**) and a value are the contents of the corresponding file. An optional [APTicket](https://www.theiphonewiki.com/wiki/APTicket) may be included as a signature of the file's contents.

## Building 'ftab' files

**ftabutil** introduces a concept of manifest — a TOML description of the 'ftab' file fields and contents. Manifests are automatically produced when unpacking 'ftab' files but can also be created manually. Here's an example of such a manifest:

```toml
# Unknown fields that are ignored by all the available parsers, required.
unk_0 = 83886336
unk_1 = 4294967295
unk_2 = 0
unk_3 = 0
unk_4 = 0
unk_5 = 0
unk_6 = 0
# Path to the ticket to be included in the 'ftab' file, optional.
ticket = "ApImg4Ticker.der"

# List of files to be included as the file's segments.
[[segments]]
# Path to the file to be included as a segment.
path = "rkos.bin"
# The tag to be assigned to the segment. Can either be a 4-byte string
# or an integer less than 2^32 that will be encoded as a big-endian 
# 32-bit integer.
tag = "rkos"
# Unknown field that is always equal to zero.
unk = 0

# ...
```

Such a manifest can be used with the `pack` subcommand like this:

```shell
ftabutil pack path/to/manifest.toml optional/path/to/ftab.bin
```

For more info see documentation for the `pack` subcommand.

## Unstable access to unknown fields

Some fields of the format are unknown and unused at the time of writing. The tool provides the access to these fields without really documenting them. In the future the names for these fields is very likely to change, so you shouldn't rely on the manifest format to be stable.

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.