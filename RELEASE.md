# Notes on releasing

Because the `bagr` version is captured in test files, the files must
be updated prior to tagging a release. The following are the steps
that should be followed when releasing a new version:

``` shell
# Move off a dev version and update the change log
cargo release --no-tag --no-push --no-publish --execute

# Update the bagr version in test files
TRYCMD=overwrite cargo test

# Fix up the release commit
git commit --amend
git tag vVERSION
git push origin main
git push origin vVERSION

cargo publish
```
