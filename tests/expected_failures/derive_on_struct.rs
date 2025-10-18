use enum_variant_accessors::{EnumAsVariant, EnumIsVariant};

#[derive(EnumIsVariant, EnumAsVariant)]
struct Person {
    name: String,
    age: i32,
}

fn main() {}
