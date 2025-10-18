use enum_variant_accessors::{EnumAsVariant, EnumIsVariant};

#[derive(EnumIsVariant, EnumAsVariant, Debug, Clone)]
enum Bag<'a, T, const N: usize>
where
    T: 'a + core::fmt::Debug,
{
    Empty,
    One(&'a T),
    Many(&'a [T; N]),
    Named { item: &'a T, count: usize },
}

#[test]
fn generics_and_where_clause() {
    let x = 7u32;
    let a = Bag::<u32, 3>::One(&x);
    assert!(a.is_one());
    assert_eq!(a.as_one(), Some(&7u32));

    let arr = [1u32, 2, 3];
    let m = Bag::<u32, 3>::Many(&arr);
    assert!(m.is_many());
    let got = m.as_many().unwrap();
    assert_eq!(got.len(), 3);

    let n = Bag::<u32, 3>::Named { item: &x, count: 2 };
    let p = n.as_named().unwrap();
    assert_eq!(**p.item, 7);
    assert_eq!(*p.count, 2);
}
