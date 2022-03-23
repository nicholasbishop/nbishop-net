+++
title: Investigating a Rust-on-UEFI division bug
date: 2022-03-22
+++

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
