+++
title: Cleaning up old Rust projects
date: 2024-03-17
+++

I recently learned about the maintainer dashboard feature of lib.rs:
[lib.rs/dash][dash]. I don't think the dashboard is linked anywhere, so
you just have to find out about it from [somewhere else][librswebimp],
but it's a very handy feature. It can point out all sorts of problems
with your crates, such as:
* Some dependency is three major versions out of date
* The crate is missing metadata like categories or keywords that might help people find it
* The crate hasn't been updated in nine years, is it still maintained?

I have created a fair number of crates over the years, most of which I
no longer actually maintain. Maybe the crate was for a hobby project
that I've since moved on from, or for a job that I left years
ago. Whatever the reason, I have certainly accumulated a lot of stale
crates, and the dashboard made it easy to see them. I spent some time
cleaning them up, which is pretty easy to do:

1. Clone the repo. If it's archived, run `gh repo unarchive -y` to unarchive it.
2. Add a note at the top of the readme:
   ```
   **This tool is no longer under active development. If you are
   interested in taking over or repurposing the name on crates.io, feel
   free to contact me: nbishop@nbishop.net**
   ```
3. In `Cargo.toml`, bump the patch version and add this section:
   ```toml
   [badges.maintenance]
   status = "deprecated"
   ```
4. Commit the changes, push, and publish the final deprecated version of the package:
   ```
   git commit -am 'Deprecate' && git push && cargo publish
   ```
5. Archive the repo: `gh repo archive -y`

Pretty straightforward, and now libs.rs will ignore the crate because
it's deprecated. Better search results for everybody, and if someone
wants to take over the name on crates.io (whether to start maintaining
the crate again, or to replace it with something else entirely), they
know who to contact.

Aside: by default, `cargo publish` compiles the crate before
publishing. I could have turned that off with `--no-verify`, but I was
curious to see if compilation would succeed on code that hasn't been
touched in years (nine years, in the longest case). Result: other than
one crate that required a system dep I didn't happen to have installed,
all crates compiled successfully. That would certainly not have been the
case with C or C++ projects. Even figuring out _how_ to compile projects
in those languages would have taken some effort. As important as memory
safety is, this is a good example of how Rust brings a lot more to the
table.

Final thought: I kinda wish that crates.io had some out-of-band method
of marking a crate as deprecated, rather than having to publish a new
version that just marks the crate as deprecated. I don't think the
current way of doing it is unreasonable, but it would be nice to be able
to go into the crates.io settings and mark a crate as deprecated
there. It would also be nice to have a built-in way to say "hey, I don't
care about owning this name anymore; feel free to send an email to
`<address>` to request it". Or maybe even a button that says "click here
to just take the name immediately". At any rate, this is just a minor
wish. The current method is not bad, just takes a bit more effort.

[dash]: https://lib.rs/dash
[librswebimp]: https://users.rust-lang.org/t/lib-rs-website-improvements/108218
