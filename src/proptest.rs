use proptest::prelude::*;

proptest! {
    #[test]
    fn test_add(a in 0..=3, b in 0..=3) {
        prop_assert!(a + b <= 6)
    }

    #[test]
    fn string_cat_length(a in ".*", b in ".*") {
        // let cat = format!({}{}, a, b);
        // prop_assert_eq!()
    }
}