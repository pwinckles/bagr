# bagr

`bagr` is a command line utility for interacting with
[BagIt](https://datatracker.ietf.org/doc/html/rfc8493) bags.

It is in early development, and not ready to be used yet.

## Not Supported

1. Files containing newline characters in their name. The [1.0
   spec](https://datatracker.ietf.org/doc/html/rfc8493#section-2.1.3)
   requires that `CR`, `LF`, and `%s` are percent encoded in
   manifests. However, I have yet to find an implementation that does
   this correctly. As such, the least broken behavior is to reject the
   creation of bags containing files with newline characters in their
   names, and do not decode the paths when reading manifests. The
   issue is documented
   [here](https://github.com/LibraryOfCongress/bagit-spec/issues/46).
2. Tag files **must** be UTF-8 encoded. Other encodings may be
   supported eventually.
3. Only BagIt version `1.0` is supported. `0.97` will likely be
   supported eventually due to its prevalence.
