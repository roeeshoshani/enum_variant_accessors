use enum_variant_accessors::{EnumAsVariant, EnumIsVariant};

#[derive(EnumIsVariant, EnumAsVariant, Debug)]
enum Node<'a> {
    Unit,
    #[allow(dead_code)]
    Empty(),
    One(i32),
    Two(&'a str, usize),
}

fn main() {
    let n = Node::Unit;
    assert!(n.is_unit());
    assert_eq!(n.as_unit(), Some(()));

    let mut m = Node::Two("x", 1);
    if let Some((s, v)) = m.as_two_mut() {
        *v += 3;
        assert_eq!(*s, "x");
    }
    assert_eq!(m.as_two(), Some((&"x", &4)));

    let k = Node::One(10);
    assert_eq!(k.as_one(), Some(&10));
}
