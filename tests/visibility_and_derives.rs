use enum_variant_accessors::{EnumAsVariant, EnumIsVariant};

mod inner {
    use super::*;

    // pub(crate) visibility
    #[derive(EnumIsVariant, EnumAsVariant, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub(crate) enum Shape<'a> {
        Point,
        Label(&'a str),
        Rect { w: u32, h: u32 },
    }

    pub fn mk() -> Shape<'static> {
        Shape::Rect { w: 2, h: 9 }
    }
}

#[test]
fn visibility_mirrors_enum() {
    // We can't name the generated struct directly if it's pub(crate) from another crate,
    // but within the same crate this should be accessible.
    let s = inner::mk();
    assert!(s.is_rect());
    let r = s.as_rect().unwrap();
    // Generated struct derives Debug & Clone (from whitelist):
    let _ = format!("{r:?}");
}

mod pubmod {
    use super::*;

    // Public enum should generate public helper structs and public fields.
    #[derive(EnumIsVariant, EnumAsVariant, Debug, Clone, PartialEq, Eq)]
    pub enum Event {
        Ping,
        Person { name: String, age: u32 },
    }

    pub fn mk() -> Event {
        Event::Person {
            name: "Ann".into(),
            age: 34,
        }
    }
}

#[test]
fn public_helper_struct_is_public() {
    let e = pubmod::mk();
    let p = e.as_person().unwrap();
    // Able to access fields (public):
    assert_eq!(p.name, "Ann");
    assert_eq!(*p.age, 34);
}
