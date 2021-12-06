# Loiter 🐢

Loiter the lazy tortoise wants you to do less and get more done when it comes to
tracking your time.

## Overview

Loiter is a time tracking tool and Rust library (yes, yet another one, because
to get the UX you want you've often got to build it yourself). It's inspired by
[Watson]. It stores its data in plain old text files that're easy to `grep`,
`jq` and commit to a Git repository.

At present, it primarily provides a command line-based interface for:

- Managing projects
- Managing tasks related to projects
- Tracking time worked on specific tasks and/or projects

## Building and Testing

Note that, due to [this unsoundness
issue](https://github.com/time-rs/time/issues/293) in the `time` crate, Loiter
must be built with the `RUSTFLAGS="--cfg unsound_local_offset"` flag set.

```bash
# Building and installing from source
RUSTFLAGS="--cfg unsound_local_offset" cargo install --path loiter-cli

# Testing
RUSTFLAGS="--cfg unsound_local_offset" cargo test
```

[Watson]: https://github.com/TailorDev/Watson

