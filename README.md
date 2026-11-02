# hashlink -- HashMap-like containers that hold their key-value pairs in a user controllable order

[![Build status](https://github.com/djc/hashlink/workflows/CI/badge.svg)](https://github.com/djc/hashlink/actions?query=workflow%3ACI)
[![Latest Version](https://img.shields.io/crates/v/hashlink.svg)](https://crates.io/crates/hashlink)
[![API Documentation](https://docs.rs/hashlink/badge.svg)](https://docs.rs/hashlink)

This crate is a fork of
[linked-hash-map](https://github.com/contain-rs/linked-hash-map) that builds on
top of [hashbrown](https://github.com/rust-lang/hashbrown) to implement more up
to date versions of `LinkedHashMap` `LinkedHashSet`, and `LruCache`.

One important API change is that when a `LinkedHashMap` is used as a LRU cache,
it allows you to easily retrieve an entry and move it to the back OR produce a
new entry at the back without needlessly repeating key hashing and lookups:

``` rust
let mut lru_cache = LinkedHashMap::new();
let key = "key".to_owned();
// Try to find my expensive to construct and hash key
let _cached_val = match lru_cache.raw_entry_mut().from_key(&key) {
    RawEntryMut::Occupied(mut occupied) => {
        // Cache hit, move entry to the back.
        occupied.to_back();
        occupied.into_mut()
    }
    RawEntryMut::Vacant(vacant) => {
        // Insert expensive to construct key and expensive to compute value,
        // automatically inserted at the back.
        vacant.insert(key.clone(), 42).1
    }
};
```

Or, a simpler way to do the same thing:

``` rust
let mut lru_cache = LinkedHashMap::new();
let key = "key".to_owned();
let _cached_val = lru_cache
    .raw_entry_mut()
    .from_key(&key)
    .or_insert_with(|| (key.clone(), 42));
```

This crate contains a decent amount of unsafe code from handling its internal
linked list, and the unsafe code has diverged quite a lot from the original
`linked-hash-map` implementation.  It currently passes tests under miri and
sanitizers, but it should probably still receive more review and testing, and
check for test code coverage.

## Testing unsafe code

The integration test `tests/proptest.rs` uses proptest to compare random operation
sequences against a linear, ordered reference model. It checks contents, lookups,
length, and forward/backward iteration after each operation, using both the default
hasher and a deliberately colliding hasher. Heap-owned keys and values exercise
destruction as well as node reuse, cursor insertion, reordering, retention,
resizing, mutable iteration, and partially consumed drains and owning iterators.
The map contains the shared unsafe linked-list implementation used by the set and
LRU cache.

Run the property tests with:

```sh
cargo test --locked --test proptest
PROPTEST_CASES=10000 cargo test --locked --test proptest
cargo +nightly miri test --locked --test proptest
```

These tests also run in the existing Miri and sanitizer CI jobs, with smaller
default case counts and sequences under Miri. Proptest shrinks failing sequences
and saves regression seeds; keep those seeds when adding a regression test.
Seed persistence is disabled under Miri to avoid filesystem access in isolation.

The `fuzz/` package uses cargo-fuzz/libFuzzer and the same reference-model runner:

```sh
cargo install cargo-fuzz --locked
cargo +nightly fuzz run linked_hash_map -- -max_len=1024
# Bounded smoke test:
cargo +nightly fuzz run linked_hash_map -- -max_len=1024 -max_total_time=30
# Replay a saved failure:
cargo +nightly fuzz run linked_hash_map fuzz/artifacts/linked_hash_map/<artifact>
```

Each four input bytes encode an operation, a key, and a little-endian `i16`
value. Inputs are limited to 256 operations and 32 distinct keys to favor
replacement and node reuse while bounding resource consumption. Generated
corpora, crash artifacts, coverage output, and fuzz build output are ignored.

## Credit

There is a huge amount of code in this crate that is copied verbatim from
`linked-hash-map` and `hashbrown`, especially tests, associated types like
iterators, and things like `Debug` impls.

## License

This library is licensed the same as
[linked-hash-map](https://github.com/contain-rs/linked-hash-map) and
[hashbrown](https://github.com/rust-lang/hashbrown), it is licensed under either
of:

* MIT license [LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT
* Apache License 2.0 [LICENSE-APACHE](LICENSE-APACHE) or https://opensource.org/licenses/Apache-2.0

at your option.
