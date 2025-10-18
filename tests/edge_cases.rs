use enum_variant_accessors::{EnumAsVariant, EnumIsVariant};

#[derive(
    EnumIsVariant, EnumAsVariant, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
enum Weird {
    // Unit
    A,
    // Single unnamed field that is already a reference
    B(&'static str),
    // Multiple unnamed with mixed types
    C(i8, bool, usize),
    // Named with field attributes
    D {
        #[allow(unused)]
        flag: bool,
        value: u64,
    },
}

#[test]
fn references_are_double_refs_as_expected() {
    let e = Weird::B("x");
    // returns &&str by reference capture, which is fine and expected.
    let got = e.as_b().unwrap();
    assert_eq!(**got, "x");
}

#[test]
fn multiple_unnamed_tuple_of_refs() {
    let e = Weird::C(-1, true, 42);
    let (a, b, c) = e.as_c().unwrap();
    assert_eq!((*a, *b, *c), (-1, true, 42));
}

#[test]
fn named_keeps_field_attrs_and_is_borrowed() {
    let e = Weird::D {
        flag: true,
        value: 9,
    };
    let d = e.as_d().unwrap();
    // Fields are references:
    assert_eq!((*d.flag, *d.value), (true, 9));
    // Derived traits on helper allow Debug:
    let _ = format!("{d:?}");
}
