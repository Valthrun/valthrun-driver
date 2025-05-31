# Valthrun Driver

This repository contains the Rust library for interacting with the **Valthrun Memory Drivers**, along with some basic implementations of memory drivers for.

> **Note:** Currently, only the **user-mode driver** is available in this repository.  
> The **kernel-mode driver** can be found in a separate repository: [valthrun-driver-kernel](https://github.com/Valthrun/valthrun-driver-kernel)

## How to use
To use the user-mode driver interface, simply add `vtd-libum` as a dependency in your `Cargo.toml`:


```toml
[dependencies]
vtd-libum = { git = "https://github.com/Valthrun/valthrun-driver", rev = "3931af3" }
```

> ⚠️ Note:  
> Be sure to update the rev field to the latest commit for the most recent changes.

## Examples
Examples demonstrating how to use the Valthrun Driver Interface to:
- Read arbitrary memory
- List processes
- Enumerate process modules

can be found in the `crates/vtd-libum/examples` directory.