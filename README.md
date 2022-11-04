# ftabutil

A simple utility to build and unpack 'ftab' (aka 'rkosftab') images found in firmware images of accessories produced by Apple.

## A quick into 'ftab' file format

'ftab' (which probably means **F**ile **TAB**le) files are simple tables of files where a key is a 4-byte tag (e.g. 'rkos' which stands for **R**T**K**it **OS**) and a value are the contents of the corresponding file. An optional [APTicket](https://www.theiphonewiki.com/wiki/APTicket) may be included as a signature of the file's contents.

## Building 'ftab' files

**ftabutil** introduces a concept of manifest — a TOML description of the 'ftab' file fields and contents. Manifests are automatically produced when unpacking 'ftab' files but can also be created manually. Here's an example of such a manifest:

```toml

```

## Unstable access to unknown fields

Some fields of the format are unknown and unused at the time