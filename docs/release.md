Building a release and publishing it are two separate steps:

```console
# Builds every dictionary into data/
$ cargo run -r -- release

# The same, also making the mdict dictionaries
$ cargo run -r -- release --format yomitan,mdict

# Uploads them to hugging face
$ just hf publish
```

`hf publish` moves `data/{dict,index}` into `data/release`, uploads both to `latest/`, then tags that commit with the release date. Only the zips of `dict/` are uploaded: the mdict ones are stored in a `mdict/` folder next to the yomitan zip of the pair.

The release writes one `docs/release_metadata_<format>.json` per format it built, each with the date of the release, which is what the [download](download.md) page reads. A format left out of a release keeps the metadata (and the date) of the last release that made it.

`latest/` is the only path that cannot move: it is baked into the `indexUrl` of every dictionary already installed in yomitan (see [update](update.md)). Tags replace the older `versions/{date}` copies of those same files, and are browsable at `{repo}/tree/{date}`. Republishing on the same day moves the tag to the new commit, so it always matches `latest/`.

If a publish is interrupted, run it again: staging is idempotent and the uploads resume where they stopped. The remaining commands are `just hf tag {list,create,delete}`, and `just hf squash`, which squashes the repo history to reclaim storage.
