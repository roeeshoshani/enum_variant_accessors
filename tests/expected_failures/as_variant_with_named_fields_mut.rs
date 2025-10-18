use enum_variant_accessors::{EnumAsVariant, EnumIsVariant};

#[derive(EnumIsVariant, EnumAsVariant)]
enum MyEnum {
    A,
    A2(),
    C(String, usize),
    D { x: u32, y: u16 },
}

fn main() {
    let x = MyEnum::D { x: 0, y: 0 };

    // as_* functions should not be generated for variants with named fields.
    let _ = x.as_named_mut();
}
