Tests are run with `cargo test`, benchmarks with `cargo bench`.

If you only want to run tests for the main dictionary in a single language pair, without capturing output:

```console
$ cargo run -- main ja en --root-dir=tests --pretty
```

To add a word (here, faul) to the testsuite, besides copy pasting it, you can run (requires [just](https://github.com/casey/just)):

```console
$ just add de en faul
```
