mod support;

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: if cfg!(miri) { 8 } else { 128 },
        failure_persistence: if cfg!(miri) {
            None
        } else {
            ProptestConfig::default().failure_persistence
        },
        ..ProptestConfig::default()
    })]

    #[test]
    fn linked_list_operations_match_model(
        operations in prop::collection::vec(
            (any::<u8>(), any::<u8>(), any::<i16>()),
            0..if cfg!(miri) { 32 } else { 256 },
        )
    ) {
        support::check_operations(&operations);
    }
}

#[test]
fn node_reuse_and_iterator_drop() {
    let mut operations = Vec::new();
    for round in 0..3 {
        for key in 0..32 {
            operations.push((0, key, round));
        }
        // Exercise every operation between growth cycles.
        for operation in 0..24 {
            operations.push((operation, operation, round));
        }
        operations.push((16, 0, 0));
    }
    support::check_operations(&operations);
}
