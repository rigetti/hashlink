use std::hash::{BuildHasher, BuildHasherDefault, Hasher};

use hashlink::{DefaultHashBuilder, LinkedHashMap, linked_hash_map::RawEntryMut};

#[derive(Default)]
struct CollidingHasher;

impl Hasher for CollidingHasher {
    fn write(&mut self, _: &[u8]) {}

    fn finish(&self) -> u64 {
        0
    }
}

pub fn check_operations(operations: &[(u8, u8, i16)]) {
    check_with_hasher::<DefaultHashBuilder>(operations);
    check_with_hasher::<BuildHasherDefault<CollidingHasher>>(operations);
}

fn check_with_hasher<S: BuildHasher + Default + Clone>(operations: &[(u8, u8, i16)]) {
    let mut map = LinkedHashMap::with_hasher(S::default());
    // A linear model avoids relying on hashlink's table or linked-list implementation.
    let mut model: Vec<(String, String)> = Vec::new();

    for &(operation, key, value) in operations {
        let key = (key % 32).to_string();
        let value = value.to_string();
        let position = model.iter().position(|(k, _)| k == &key);

        match operation % 24 {
            0 | 1 => {
                let old = position.map(|i| model.remove(i).1);
                let actual = if operation % 24 == 0 {
                    model.push((key.clone(), value.clone()));
                    map.insert(key.clone(), value)
                } else {
                    model.insert(0, (key.clone(), value.clone()));
                    map.insert_front(key.clone(), value)
                };
                assert_eq!(actual, old);
            }
            2 => {
                let old = if let Some(i) = position {
                    Some(std::mem::replace(&mut model[i].1, value.clone()))
                } else {
                    model.push((key.clone(), value.clone()));
                    None
                };
                assert_eq!(map.replace(key.clone(), value), old);
            }
            3 => {
                assert_eq!(
                    map.remove_entry(key.as_str()),
                    position.map(|i| model.remove(i))
                );
            }
            4 => {
                let expected = if model.is_empty() {
                    None
                } else {
                    Some(model.remove(0))
                };
                assert_eq!(map.pop_front(), expected);
            }
            5 => assert_eq!(map.pop_back(), model.pop()),
            6 | 7 => {
                let expected = position.map(|i| {
                    let entry = model.remove(i);
                    let value = entry.1.clone();
                    if operation % 24 == 6 {
                        model.insert(0, entry);
                    } else {
                        model.push(entry);
                    }
                    value
                });
                let actual = if operation % 24 == 6 {
                    map.to_front(key.as_str())
                } else {
                    map.to_back(key.as_str())
                };
                assert_eq!(actual.cloned(), expected);
            }
            8 => {
                let expected = if let Some(i) = position {
                    let entry = model.remove(i);
                    let old = entry.1.clone();
                    model.push(entry);
                    old
                } else {
                    model.push((key.clone(), value.clone()));
                    value.clone()
                };
                assert_eq!(*map.entry(key.clone()).or_insert(value), expected);
            }
            9 => match map.raw_entry_mut().from_key(key.as_str()) {
                RawEntryMut::Occupied(entry) => {
                    assert_eq!(
                        Some(entry.remove_entry()),
                        position.map(|i| model.remove(i))
                    );
                }
                RawEntryMut::Vacant(entry) => {
                    assert!(position.is_none());
                    entry.insert(key.clone(), value.clone());
                    model.push((key.clone(), value));
                }
            },
            10 | 11 => {
                let keep = |k: &String, v: &mut String| {
                    v.push('r');
                    k.as_bytes()[0] % 2 == key.as_bytes()[0] % 2
                };
                if operation % 24 == 10 {
                    map.retain(keep);
                } else {
                    map.retain_with_order(keep);
                }
                model.retain_mut(|(k, v)| keep(k, v));
            }
            12 => {
                // Keep all mutable references alive together to expose iterator aliasing in Miri.
                let mut iter = map.iter_mut();
                let mut entries = Vec::new();
                for i in 0..model.len() {
                    entries.push(if i % 2 == 0 {
                        iter.next().unwrap()
                    } else {
                        iter.next_back().unwrap()
                    });
                }
                assert!(iter.next().is_none());
                assert!(iter.next_back().is_none());
                for (_, v) in entries {
                    v.push('m');
                }
                for (_, v) in &mut model {
                    v.push('m');
                }
            }
            13 => {
                // Dropping a partially consumed drain must empty the map and leave it reusable.
                let mut drain = map.drain();
                if !model.is_empty() {
                    assert_eq!(drain.next(), Some(model.remove(0)));
                }
                assert_eq!(drain.next_back(), model.pop());
                drop(drain);
                model.clear();
            }
            14 => {
                map.clear();
                model.clear();
            }
            15 => map.reserve(usize::from(operation % 32)),
            16 => map.shrink_to_fit(),
            17 => {
                let cloned = map.clone();
                assert!(cloned.iter().eq(model.iter().map(|(k, v)| (k, v))));
                let mut iter = cloned.into_iter();
                let mut expected = model.clone();
                if !expected.is_empty() {
                    assert_eq!(iter.next(), Some(expected.remove(0)));
                }
                assert_eq!(iter.next_back(), expected.pop());
                // Leave the rest to IntoIter::drop.
            }
            18 | 19 => {
                // Include the ghost position and insertion next to an existing key (including itself).
                let index = usize::from(operation) % (model.len() + 1);
                let before = if operation % 24 == 18 {
                    model.get(index)
                } else if index == model.len() {
                    model.first()
                } else {
                    model.get(index + 1)
                }
                .map(|(k, _)| k.clone());
                let mut cursor = map.cursor_front_mut();
                for _ in 0..index {
                    cursor.move_next();
                }
                let old = if operation % 24 == 18 {
                    cursor.insert_before(key.clone(), value.clone())
                } else {
                    cursor.insert_after(key.clone(), value.clone())
                };
                let expected = position.map(|i| model.remove(i).1);
                let index = before
                    .as_ref()
                    .and_then(|k| model.iter().position(|(other, _)| other == k))
                    .unwrap_or(model.len());
                // Inserting immediately before itself only replaces the value.
                let index = if before.as_ref() == Some(&key) {
                    position.unwrap()
                } else {
                    index
                };
                model.insert(index, (key.clone(), value));
                assert_eq!(old, expected);
            }
            20 => {
                if let Some(v) = map.get_mut(key.as_str()) {
                    *v = value.clone();
                }
                if let Some(i) = position {
                    model[i].1 = value;
                }
            }
            21 => {
                let mut cursor = map.cursor_back_mut();
                for (k, v) in model.iter().rev() {
                    assert_eq!(cursor.current().map(|(k, v)| (k, &*v)), Some((k, v)));
                    cursor.move_prev();
                }
                assert!(cursor.current().is_none());
            }
            22 => {
                let expected = if model.is_empty() {
                    None
                } else {
                    Some(model.remove(0))
                };
                assert_eq!(
                    map.front_entry().map(|entry| entry.remove_entry()),
                    expected
                );
            }
            23 => {
                assert_eq!(
                    map.back_entry().map(|entry| entry.remove_entry()),
                    model.pop()
                );
            }
            _ => unreachable!(),
        }

        assert_eq!(map.len(), model.len());
        assert_eq!(map.is_empty(), model.is_empty());
        assert_eq!(map.front(), model.first().map(|(k, v)| (k, v)));
        assert_eq!(map.back(), model.last().map(|(k, v)| (k, v)));
        assert!(
            map.iter()
                .take(model.len() + 1)
                .eq(model.iter().map(|(k, v)| (k, v)))
        );
        assert!(
            map.iter()
                .rev()
                .take(model.len() + 1)
                .eq(model.iter().rev().map(|(k, v)| (k, v)))
        );
        for (k, v) in &model {
            assert_eq!(map.get(k.as_str()), Some(v));
        }
        assert_eq!(
            map.get(key.as_str()),
            model.iter().find(|(k, _)| k == &key).map(|(_, v)| v)
        );
    }
}
