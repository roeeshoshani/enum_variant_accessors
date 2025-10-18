use enum_variant_accessors::{EnumAsVariant, EnumIsVariant};

#[derive(EnumIsVariant, EnumAsVariant)]
enum E<'a, T>
where
    T: 'a,
{
    A,
    A2(),
    B(T),
    C(&'a str, usize),
}

#[test]
fn test_is_methods() {
    let x = E::<i32>::A;
    assert!(x.is_a());
    assert!(!x.is_b());
    assert!(!x.is_c());

    let y = E::<i32>::A2();
    assert!(y.is_a2());
}

#[test]
fn test_as_unit() {
    let a = E::<i32>::A;
    assert_eq!(a.as_a(), Some(()));
    assert_eq!(a.as_a2(), None);

    let a2 = E::<i32>::A2();
    assert_eq!(a2.as_a(), None);
    assert_eq!(a2.as_a2(), Some(()));
}

#[test]
fn test_as_single() {
    let b = E::<i32>::B(5);
    assert_eq!(b.as_b(), Some(&5));
    assert_eq!(b.as_c(), None);
}

#[test]
fn test_as_multi() {
    let mut c = E::<i32>::C("ok", 9);
    assert_eq!(c.as_c(), Some((&"ok", &9)));
    if let Some((s, n)) = c.as_c_mut() {
        assert_eq!(*s, "ok");
        *n += 1;
    }
    assert_eq!(c.as_c(), Some((&"ok", &10)));
}

#[test]
fn test_generics_bounds() {
    #[derive(EnumIsVariant, EnumAsVariant)]
    enum G<'a, U: 'a> {
        X(&'a U),
        Y,
    }
    let v = 3u8;
    let g = G::X(&v);
    assert!(g.is_x());
    assert_eq!(g.as_x(), Some(&&3u8));
}
