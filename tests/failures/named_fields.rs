use enum_variant_accessors::{EnumAsVariant, EnumIsVariant};

#[derive(EnumIsVariant, EnumAsVariant)]
enum Bad {
    // Named-field variants are unsupported and should fail
    Named { x: i32, y: i32 },
}

fn main() {}
