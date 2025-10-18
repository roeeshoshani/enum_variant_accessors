# enum_variant_accessors

Derive macros that generate ergonomic variant accessors for enums.

- `#[derive(EnumIsVariant)]` — adds `is_<variant>() -> bool` methods.
- `#[derive(EnumAsVariant)]` — adds borrowed accessors:
  - `as_<variant>(&self) -> Option<<borrowed variant data>>`
  - `as_<variant>_mut(&mut self) -> Option<<borrowed variant data (mutable)>>`

## Supported variants

- **Unit** variants (`Variant` or `Variant()`):
  - Borrowed data type: `()`
  - Accessors return `Option<()>`
- **Single-field tuple** variants (`Variant(T)`):
  - Borrowed data type: `&T` / `&mut T`
  - Accessors return `Option<&T>` / `Option<&mut T>`
- **Multi-field tuple** variants (`Variant(T1, T2, ...)`):
  - Borrowed data type: `(&T1, &T2, ...)` / `(&mut T1, &mut T2, ...)`
  - Accessors return `Option<(&T1, &T2, ...)>` / `Option<(&mut T1, &mut T2, ...)>`

> **Not supported:** generating `as_*` functions for named-field (struct-like) variants. such variants will be skipped when generating `as_*` functions.

## Examples

```rust
use enum_variant_accessors::{EnumIsVariant, EnumAsVariant};

#[derive(EnumIsVariant, EnumAsVariant)]
enum Msg<'a> {
    Ping,
    Pong(),
    One(u32),
    Pair(&'a str, usize),
}

fn main() {
    let mut m = Msg::Pair("hi", 7);

    assert!(m.is_pair());
    assert!(!m.is_ping());
    assert!(matches!(m.as_ping(), None));
    assert_eq!(m.as_pair(), Some((&"hi", &7)));

    if let Some((s, n)) = m.as_pair_mut() {
        *n += 1;
        assert_eq!(*s, "hi");
    }
    assert_eq!(m.as_pair(), Some((&"hi", &8)));

    assert_eq!(m.as_pong(), None);
    let p = Msg::Pong();
    assert_eq!(p.as_pong(), Some(()));
}
```
