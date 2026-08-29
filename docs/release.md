Building a release and publishing it are two separate steps:

```console
$ just release              # builds every dictionary into data/release
$ just hf publish           # uploads them to the hugging face dataset
```

`hf publish` moves `data/{dict,index}` into `data/release`, uploads both to `latest/`, then tags that commit with the release date.

`latest/` is the only path that cannot move: it is baked into the `indexUrl` of every dictionary already installed in yomitan (see [update](update.md)). Tags replace the older `versions/{date}` copies of those same files, and are browsable at `{repo}/tree/{date}`. Republishing on the same day moves the tag to the new commit, so it always matches `latest/`.

If a publish is interrupted, run it again: staging is idempotent and the uploads resume where they stopped. The remaining commands are `just hf tag {list,create,delete}`, and `just hf squash`, which squashes the repo history to reclaim storage.
