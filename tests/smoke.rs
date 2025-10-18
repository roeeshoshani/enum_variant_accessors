use enum_variant_accessors::{EnumAsVariant, EnumIsVariant};

#[derive(EnumIsVariant, EnumAsVariant, Debug, Clone, PartialEq, Eq)]
enum Msg<T> {
    Unit,
    One(T),
    Two(u8, u16),
    Record { x: usize, y: usize },
}

#[test]
fn smoke_is_and_as() {
    let m = Msg::Unit::<String>;
    assert!(m.is_unit());
    assert_eq!(m.as_unit(), Some(()));

    let s = Msg::One("hello".to_string());
    assert!(s.is_one());
    let r = s.as_one().unwrap();
    assert_eq!(r, "hello");

    let p = Msg::Two(3, 5);
    let pair = p.as_two().unwrap();
    assert_eq!((*pair.0, *pair.1), (3u8, 5u16));

    let rec = Msg::Record { x: 10, y: 20 };
    let got = rec.as_record().unwrap();
    assert_eq!((*got.x, *got.y), (10, 20));
}
