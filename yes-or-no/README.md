# yes-or-no

[![crates.io badge](https://img.shields.io/crates/v/yes-or-no.svg?label=yes-or-no)](https://crates.io/crates/yes-or-no)
[![docs.rs badge](https://docs.rs/yes-or-no/badge.svg)](https://docs.rs/yes-or-no)
[![CI Status](https://github.com/BrainiumLLC/yes-or-no/workflows/CI/badge.svg)](https://github.com/BrainiumLLC/yes-or-no/actions)

A macro that generates an enum with the variants `Yes` and `No`.

```rust
use yes_or_no::yes_or_no;

yes_or_no!(Hungry);

assert_eq!(Hungry::from(true), Hungry::Yes);
assert_eq!(Hungry::from(false), Hungry::No);
assert!(Hungry::Yes.yes());
assert!(Hungry::No.no());
```
