# ykbf: Yorick BF intererpter

To build, run, and test ykbf, use regular Cargo commands, but set the `YK_DIR`
environment to point to a (compiled) `yk` repo and substitute invocations of
`cargo` with `./cargo-hwtracer`.

E.g.
```
$ export YK_DIR=~/research/yorick/yk
$ ./cargo-hwtracer build
```
