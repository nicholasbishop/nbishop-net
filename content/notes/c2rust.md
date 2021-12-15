+++
title = "Building c2rust"
date = 2021-06-05
+++

[c2rust](https://github.com/immunant/c2rust) is a really neat and
useful tool :) Unfortunately I had some difficulty building c2rust,
here are some notes on how I got it to work.

The c2rust build depends on the host environment, so basically you
don't want to build it there or it will break. Building it in a
container is the way to go. The c2rust repo has a docker subdirectory,
but for some reason it just seems to be for setting up a build
environment to build c2rust; it doesn't go all the way and just give
you the binary. And the instructions don't actually say what to do
with the scripts there, at least as far as I can tell (I may have
missed something!), so it's all a bit confusing.

Here's what worked for me on Fedora:

1. Create a `Containerfile`:

   ```docker
   FROM docker://immunant/c2rust:archlinux-base-latest

   RUN git clone https://github.com/immunant/c2rust.git
   WORKDIR c2rust
   RUN cargo +nightly-2019-12-05 build --release --locked
   
   ENTRYPOINT ["/c2rust/target/release/c2rust"]
   ```

2. Build it (this will take a while):

   ```
   podman build -f Containerfile -t c2rust
   ```
   
3. Run it in the container:

   ```
   podman run -v$PWD:/host:z c2rust transpile --output-dir /host/transpile/ /host/compile_commands.json
   ```

That last step assumes that you have a `compile_commands.json` in the
current directory and that the paths it references are under
`/host`. The output will go to `/host/transpile`.
