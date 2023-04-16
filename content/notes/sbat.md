+++
title: New project: sbat-rs
date: 2023-04-16
+++

This week I open-sourced some Rust code for working with UEFI
[SBAT](https://github.com/rhboot/shim/blob/main/SBAT.md):
<https://github.com/google/sbat-rs>

There are two crates in this project:
* [sbat](https://crates.io/crates/sbat) - A no-std library for parsing SBAT and doing revocation checks.
* [sbat-tool](https://crates.io/crates/sbat-tool) - A command-line utility for working with SBAT.
