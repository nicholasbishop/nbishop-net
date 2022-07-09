+++
title: Investigating a Rust-on-UEFI division bug
date: 2022-03-22
+++

## 2022-03-22

These are some notes related to
<https://github.com/rust-lang/rust/issues/86494>. I filed that bug a
while ago but haven't followed up on it. The bug still repros so I
thought I'd take a look and see if I can give better information about
what's going wrong. I haven't done any work on the rust compiler itself
before, so I'll probably just be flailing around a bit.

It was suggested in the bug that the invalid instruction was maybe
caused by SSE being used, which wouldn't normally be allowed in a UEFI
environment. I think though that the invalid instruction is actually not
happening in the division per se, but rather due to hitting
`unreachable_unchecked` because it somehow thinks a division by zero is
occurring.

I suspect the problem is in `compiler_builtins`,
although it's certainly possible that rustc or llvm is at fault.

It's [not currently possible](https://github.com/rust-lang/wg-cargo-std-aware/issues/61) 
to use a custom `compiler_builtins` in your project's `Cargo.toml` in
combination with `build-std`. The current recommendation is to build
rust with the necessary customizations.

So, I have both `rust` and `compiler_builtins` checked out. I added this
to `Cargo.toml` in rust:

```toml
[patch.crates-io]
compiler_builtins = { path = "../compiler-builtins" }
```

And ran `cargo update -p compiler_builtins` for good measure, not sure
if that's needed or not.

I set up `config.toml` like this, not at all sure if these are good
settings.

```toml
profile = "user"
changelog-seen = 2

[llvm]
download-ci-llvm = true

[install]
prefix = "/var/home/nbishop/src/rust/bishinstall"

[build]
target = ["x86_64-unknown-uefi"]
```

Aside: there's a lot of documentation, both in the `rust` repo and
linked to from it, e.g. the [Guide to Rustc
Development](https://rustc-dev-guide.rust-lang.org/), but I've found it
a bit challenging to navigate and find what I'm looking for. Certainly a
much better problem to have than not having documentation! But it's
still probably not as easy as it could be to get started with changing
things in Rust and I have to imagine that most people who give it a try
will bounce off the complexity pretty quickly and give up. (I may join
that group after failing at this for a bit, we'll see!) This is a
challenge that every big project faces of course, so absolutely no
judgement to the Rust folks on this, just noting there's more work to
do here.

Next, build stuff and see what breaks...

```
./x.py build
```

Lots of errors in `compiler_builtins`. I'll try checking out the version
that rust is currently pinned to `0.1.70` and build again. And that
fails the same way, hmm.

Ok, changing my `config.toml` to the `library` profile instead of
`user`, and then adding `#![feature(restricted_std)]` in
`library/test/src/lib.rs` got the build step to succeed. Then I ran
`./x.py install` to hopefully install it into the custom
prefix. Unexpectedly to me, that sent it back to compiling a bunch of
stuff instead of installing anything, and then it failed with the same
errors as before.

Ok, let's try throwing `#![allow(unexpected_cfgs)]` into
`compiler_builtins`'s `lib.rs`. Now it's blowing up in a whole new way:

```
thread 'rustc' panicked at 'no entry found for key', compiler/rustc_metadata/src/rmeta/decoder/cstore_impl.rs:525:9
```

Hmm. Maybe it's time to take a step back and do a clean build of
everything with a more stock config, because I have no idea what's going
wrong. I'll switch from my laptop to my desktop for quicker builds.

## 2022-07-09

The error I hit above was related to this issue: https://github.com/rust-lang/rust/issues/97322

Using the patch in that bug I'm able to get past the error, and now `./x.py build` succeeds. Hooray!

To get `./x.py install` to work I had to change the `sysconfdir` to be
relative, turn off docs, and limit the set of tools being built. Now my
config looks like this:

```toml
profile = "user"
changelog-seen = 2

[llvm]
download-ci-llvm = true

[install]
prefix = "/var/home/nbishop/src/rust/bishinstall"
sysconfdir = "etc"

[build]
target = ["x86_64-unknown-uefi"]
docs = false
tools = ["cargo"]
```

Now it installs. Hooray! Unfortunately, compiling an actual project
fails because `build.rs`'s tend to expect `std` to be available. So I
guess maybe I need to build the `x86_64-unknown-linux-gnu` target in
addition to the uefi target?

Changed config to:
```toml
profile = "user"
changelog-seen = 2

[llvm]
download-ci-llvm = true

[install]
prefix = "/var/home/nbishop/src/rust/bishinstall"
sysconfdir = "etc"

[build]
target = ["x86_64-unknown-linux-gnu", "x86_64-unknown-uefi"]
docs = false
tools = ["cargo"]
```

Reran `./x.py build` and `./x.py install`.

Those succeed, and I get a bit further with building a project, but it
fails at the link stage:

```
error: linker `rust-lld` not found
  |
  = note: No such file or directory (os error 2)

note: the msvc targets depend on the msvc linker but `link.exe` was not found
```

Let's try adding this to the config:

```toml
[rust]
lld = true
```

Now `./x.py build` fails with:
```
CMake Error: The source directory "/var/home/nbishop/src/rust/src/llvm-project/lld" does not exist.
```

Let's try setting `download-ci-llvm = false` in the config. That's a big
change so `./x.py build` takes a long time, but it succeeds as does `install`.

Final config:
```toml
profile = "user"
changelog-seen = 2

[llvm]
download-ci-llvm = false

[install]
prefix = "/var/home/nbishop/src/rust/bishinstall"
sysconfdir = "etc"

[build]
target = ["x86_64-unknown-linux-gnu", "x86_64-unknown-uefi"]
docs = false
tools = ["cargo"]

[rust]
lld = true
```

Rust repo commit: 6c20ab744b0f82646d90ce9d25894823abc9c669

And now it works; I can build [uefi-div-bug](git@github.com:nicholasbishop/uefi-div-bug.git) with:
```
PATH="/var/home/nbishop/src/rust/bishinstall/bin/:$PATH" ./run.py 
```

At this point I remember that I forgot about overriding the
`compiler_builtins` dependency. I try adding it in as described above,
using `patch.crates-io`, and get a bunch of `unexpected-cfgs`
errors. Let's try this:

```diff
diff --git a/src/bootstrap/builder.rs b/src/bootstrap/builder.rs
index fa6a5ee1668..ff59038fe18 100644
--- a/src/bootstrap/builder.rs
+++ b/src/bootstrap/builder.rs
@@ -1471,7 +1471,7 @@ pub fn cargo(
         // is made to work with `--check-cfg` which is currently not easly possible until cargo
         // get some support for setting `--check-cfg` within build script, it's the least invasive
         // hack that still let's us have cfg checking for the vast majority of the codebase.
-        if stage != 0 {
+        if stage == 999 {
             // Enable cfg checking of cargo features for everything but std and also enable cfg
             // checking of names and values.
             //
```

New error, needed to initialize submodules in `compiler_builtins`.

With that fixed, the build works.

`compiler_builtins` commit: 3872a7c38c64279374b46bed5c8dec45e0a5b4fd

So in theory, I can now make changes in compiler_builtins to try and fix the bug.
