## xmlwriter
![Build Status](https://github.com/RazrFalcon/xmlwriter/workflows/xmlwriter/badge.svg)
[![Crates.io](https://img.shields.io/crates/v/xmlwriter.svg)](https://crates.io/crates/xmlwriter)
[![Documentation](https://docs.rs/xmlwriter/badge.svg)](https://docs.rs/xmlwriter)
[![Rust 1.32+](https://img.shields.io/badge/rust-1.32+-orange.svg)](https://www.rust-lang.org)
![](https://img.shields.io/badge/unsafe-forbidden-brightgreen.svg)

A simple, streaming, partially-validating XML writer that writes XML data to a
std::io::Write implementation.

### Features

- A simple, bare-minimum API that panics when writing invalid XML.
- Non-allocating API. All methods are accepting either `fmt::Display` or `fmt::Arguments`.
- Nodes auto-closing.

### Example

```rust
use xmlwriter::*;
use std::io;

fn main() -> io::Result<()> {
    let opt = Options {
        use_single_quote: true,
        ..Options::default()
    };

    let mut w = XmlWriter::new(Vec::<u8>::new(), opt);
    w.start_element("svg")?;
    w.write_attribute("xmlns", "http://www.w3.org/2000/svg")?;
    w.write_attribute_fmt("viewBox", format_args!("{} {} {} {}", 0, 0, 128, 128))?;
    w.start_element("text")?;
    // We can write any object that implements `fmt::Display`.
    w.write_attribute("x", &10)?;
    w.write_attribute("y", &20)?;
    w.write_text_fmt(format_args!("length is {}", 5))?;

    assert_eq!(std::str::from_utf8(w.end_document()?.as_slice())
        .expect("xmlwriter always writes valid UTF-8"),
"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 128 128'>
    <text x='10' y='20'>
        length is 5
    </text>
</svg>
"
    );
    Ok(())
}
```

### License

MIT
