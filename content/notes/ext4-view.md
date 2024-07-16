+++
title: New project: ext4-view-rs
date: 2024-07-16
+++

I recently open-sourced and published a Rust crate for reading data from
an [ext4] file system: [ext4-view-rs]. As the name hopefully indicates,
this library is purely for reading, write support is intentionally not
included. The library is `no_std`, but does require an allocator.

The specific use I have for this code is loading data in a [UEFI]
application. Traditionally, if a UEFI app wants to read data written by
an OS, the [EFI System Partition (ESP)][ESP] is used. The ESP is
(almost) always a FAT filesystem, and UEFI has built-in support for
reading FAT filesystems. However, there are some problems with using the
ESP:
1. FAT is not a robust filesystem. If the system crashes or loses power
   partway through a write operation, the filesystem might not be
   readable on the next boot.
2. The ESP is a critical component of boot; if something goes wrong with
   the ESP, you might not be able to boot into the OS.
3. The ESP might not be very big. Many OS installs have fairly small
   ESPs, and are now bumping into disk-space limits. Modifying the
   partition layout after installation is often difficult. See for
   example Fedora's proposal to start creating a [bigger ESP][bigger-esp],
   but only for fresh installs.
   
There are various workarounds, for example maybe you have space in your
partition layout for a big ESP, or space to create a non-ESP
FAT-formatted partition (although neither of those solves the robustness
issue). But for my particular use case, a big ext4 partition was already
available, and I figured that as long as I only need to write from the
OS and read from UEFI, it shouldn't be _that_ hard to get it working.

Turns out it was not exactly trivial; ext4 hails from a time when many
developers weren't that good about documentation (even in 2024 the Linux
kernel still does not have a good culture around documentation or
testing), and ext4 also has some intrinsic design constraints due to its
upgrade path from ext2 and ext3. The [kernel docs][ext4-docs] have a lot
of good information about the data structures, but the documentation is
not complete and not always clear about edge cases. (I'm hesitant to
contribute improvements because I don't like the kernel's email-based
patch system, and I don't like that the kernel's review culture is based
on conflict rather than respect.) A fair amount of experimentation and
poking at real filesystems was necessary to sort out the details. I also
made use of some excellent [blog posts][ext4-blogs] by Srivathsa Dara
that dive into more details.

To add some extra complication, the library is licensed under MIT +
Apache-2.0 (as is standard for the Rust ecosystem), so I was careful to
implement everything myself and not use anything from GPL'd code.

Thanks to [Ted Brandston](https://ted.brandston.net/) for doing a ton of
code review for this project.

The library has had a couple releases so far and already has more or
less the final version of the API, which sticks pretty close to the
read-only parts of [`std::fs`]. There are some performance improvements
coming (in particular, faster directory entry lookup), and some features
are not yet implemented (e.g. extended attributes). Most of the pending
work can be seen in this [WIP PR]. Overall though, the library should be
quite usable, and since it's read-only you don't have to worry about the
library ever causing data loss. Bug reports and other contributions
welcome!

[ESP]: https://en.wikipedia.org/wiki/EFI_system_partition
[UEFI]: https://en.wikipedia.org/wiki/UEFI
[WIP PR]: https://github.com/nicholasbishop/ext4-view-rs/pull/113
[`std::fs`]: https://doc.rust-lang.org/std/fs/index.html]
[bigger-esp]: https://fedoraproject.org/wiki/Changes/BiggerESP
[ext4-blogs]: https://blogs.oracle.com/authors/srivathsa-dara
[ext4-docs]: https://www.kernel.org/doc/html/latest/filesystems/ext4/
[ext4-view-rs]: https://github.com/nicholasbishop/ext4-view-rs/
[ext4]: https://en.wikipedia.org/wiki/Ext4
[tbr]: https://ted.brandston.net/
