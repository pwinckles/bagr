# bagr

`bagr` is a command line utility for interacting with
[BagIt](https://datatracker.ietf.org/doc/html/rfc8493) bags.

It is still under active development, but bag creation is feature
complete.

## Install

### Pre-built

The [releases page](https://github.com/pwinckles/bagr/releases) has
pre-built binaries that should work on common OSes and architectures.
You do _not_ need to install Rust to use them.

1. Download and unzip the appropriate binary
2. Execute `./bagr help` to verify it works

### Local Build

1. Install [Rust](https://www.rust-lang.org/tools/install)
2. Execute: `cargo install bagr`
3. Verify the install: `bagr help`

## Usage

### Create a new bag

By default, `bagr` will turn the contents of the current directory
into a bag by invoking:

``` shell
bagr bag .
```

If instead, you'd like to create a bag by _copying_ the contents of a
source directory into a destination bag, then you can do by invoking:

``` shell
bagr bag src/dir dst/bag
```

By default, `sha512` is used; this algorithm can be changed using the
`--digest-algorithm` option.

On Mac systems, `.DS_Store` files can often sneak into unwanted
places. These files can be excluded from the bagging process by using
the `--exclude-hidden-files` flag. _Note_ this will **delete** hidden
files when creating a bag in place.

### Update an existing bag

If you've modified the payload or tag files of a bag after creating
it, `bagr` can also be used to recompute all of the digests and update
the appropriate manifest files by executing the following:

``` shell
bagr rebag path/to/bag
```

By default, it will use the same digest algorithms as were originally
used in the bag. If you wish to change the algorithms, you can do so
by specifying the `--digest-algorithm` option.

## Limitations

1. Tag files _must_ be UTF-8 encoded
2. `fetch.txt` is not supported
3. BagIt versions prior to 1.0 are not supported

## Roadmap

1. Implement bag validation
2. Support `fetch.txt`
3. Support BagIt 0.97
4. Support BagIt Profiles
5. Support non-UTF-8 character encodings
