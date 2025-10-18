# enum_variant_accessors

Two tiny derive macros for enums:

- `#[derive(EnumIsVariant)]` → `is_<variant>(&self) -> bool`
- `#[derive(EnumAsVariant)]` → `as_<variant>(&self) -> Option<VariantData>`

## What `as_*` returns

Because methods take `&self`, the data is **borrowed**:

- **Unit variant**: `Option<()>`
- **Single unnamed field**: `Option<&T>`
- **Multiple unnamed fields**: `Option<( &T1, &T2, ... )>`
- **Named fields**: `Option<EnumNameVariantName<'_ , ...>>` where a helper struct is generated (borrowing each field).  
  Example:
  ```rust
  #[derive(EnumAsVariant)]
  enum MyEnum {
      Person { name: String, age: u32 }
  }
  // Generated:
  pub struct MyEnumPerson<'a> {
      pub name: &'a String,
      pub age: &'a u32
  }
  impl MyEnum {
      fn as_person(&self) -> Option<MyEnumPerson<'_>> { ... }
  }

