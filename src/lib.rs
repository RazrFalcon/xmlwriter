/*!
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
*/

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_copy_implementations)]

use std::fmt::{self, Display, Write as FmtWrite};
use std::io::{self, Write};

/// An XML node indention.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Indent {
    /// Disable indention and new lines.
    None,
    /// Indent with spaces. Preferred range is 0..4.
    Spaces(u8),
    /// Indent with tabs.
    Tabs,
}

/// An XML writing options.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Use single quote marks instead of double quote.
    ///
    /// # Examples
    ///
    /// Before:
    ///
    /// ```text
    /// <rect fill="red"/>
    /// ```
    ///
    /// After:
    ///
    /// ```text
    /// <rect fill='red'/>
    /// ```
    ///
    /// Default: disabled
    pub use_single_quote: bool,

    /// Set XML nodes indention.
    ///
    /// # Examples
    ///
    /// `Indent::None`
    /// Before:
    ///
    /// ```text
    /// <svg>
    ///     <rect fill="red"/>
    /// </svg>
    /// ```
    ///
    /// After:
    ///
    /// ```text
    /// <svg><rect fill="red"/></svg>
    /// ```
    ///
    /// Default: 4 spaces
    pub indent: Indent,

    /// Set XML attributes indention.
    ///
    /// # Examples
    ///
    /// `Indent::Spaces(2)`
    ///
    /// Before:
    ///
    /// ```text
    /// <svg>
    ///     <rect fill="red" stroke="black"/>
    /// </svg>
    /// ```
    ///
    /// After:
    ///
    /// ```text
    /// <svg>
    ///     <rect
    ///       fill="red"
    ///       stroke="black"/>
    /// </svg>
    /// ```
    ///
    /// Default: `None`
    pub attributes_indent: Indent,
}

impl Default for Options {
    #[inline]
    fn default() -> Self {
        Options {
            use_single_quote: false,
            indent: Indent::Spaces(4),
            attributes_indent: Indent::None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum State {
    Empty,
    Document,
    Attributes,
    CData,
}

struct DepthData<'a> {
    element_name: Option<&'a str>,
    has_children: bool,
}

// This wrapper writer is so that we can make sure formatted strings are properly escaped too,
// as we don't have access to the formatting stuff without a fmt::Write implementation, so
// we provide it by wrapping the writer given to us while escaping appropriately any string to
// be written, depending on the type of node we're writing.
struct FmtWriter<W: Write> {
    writer: W,
    error_kind: Option<io::ErrorKind>,
    // Set to None once the text is written, as a way to make sure the code
    // sets the proper escaping type before using the fmt_writer.write_str().
    escape: Option<Escape>,
    // Same as for Options, but kept available for write_escaped()
    use_single_quote: bool,
}

impl<W: Write> FmtWriter<W> {
    fn take_err(&mut self) -> io::Error {
        let error_kind = self
            .error_kind
            .expect("there must have been an error before calling take_err()!");
        // This avoids forgetting to set it to the appropriate value when calling write_fmt().
        // We can't do it in FmtWriter's write_str(), since with a real format string the method
        // will be called several times so it'll fail in the expect() below as we'll have set
        // self.escape back to None.
        self.escape = None;
        // Make sure we can detect if take_err() is called without having an error that happened beforehand
        self.error_kind = None;

        // There's just no way of properly copying the io::Error (no Copy or Clone available), so
        // we have no choice to create a new one, which likely loses the backtrace up to this point.
        io::Error::from(error_kind)
    }

    fn write_escaped(&mut self, s: &str, escape_quotes: bool) -> io::Result<()> {
        let mut part_start_pos = 0;
        for (byte_pos, byte) in s.bytes().enumerate() {
            let escaped_char: Option<&[u8]> = match byte {
                b'&' => Some(b"&amp;"),
                b'>' => Some(b"&gt;"),
                b'<' => Some(b"&lt;"),
                b'"' if escape_quotes && !self.use_single_quote => Some(b"&quot;"),
                b'\'' if escape_quotes && self.use_single_quote => Some(b"&apos;"),
                _ => None,
            };
            if let Some(escaped_char) = escaped_char {
                // We have a character to escape, so write the previous part and the escaped character
                self.writer
                    .write_all(&s[part_start_pos..byte_pos].as_bytes())?;
                self.writer.write_all(escaped_char)?;
                // +1 skips the escaped character from part, for afterwards
                part_start_pos = byte_pos + 1;
            }
            // There's nothing to be done if the character doesn't need to be escaped, as we'll either
            // wait until we get an escapable character, or wait until the end of the string where we'll
            // just write out the rest of the string.
        }
        // Write the rest of the string which needs no escaping
        self.writer.write_all(&s[part_start_pos..].as_bytes())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Escape {
    Comment,
    AttributeValue,
    Text,
    CData,
}

impl<W: Write> fmt::Write for FmtWriter<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let error = match self
            .escape
            .expect("You must have set self.escape to Some(…) before using the formatter!")
        {
            Escape::AttributeValue => self.write_escaped(s, true),
            Escape::Text => self.write_escaped(s, false),
            // We don't bother escaping double hyphen (--) in comment as it's
            // unlikely to ever happen, and even libxml2 does not do it.
            Escape::Comment => self.writer.write_all(s.as_bytes()),
            Escape::CData => self.writer.write_all(s.as_bytes()),
        };
        if error.is_err() {
            self.error_kind = Some(error.as_ref().unwrap_err().kind());
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

/// An XML writer.
pub struct XmlWriter<'a, W: Write> {
    // When you control what you're writing enough that you know the bytes are already escaped or
    // don't need escaping at all, then use fmt_writer.writer.write_all()?; directly. Otherwise,
    // set fmt_writer.escape to the appropriate escaping type and use fmt_writer.write_fmt()?; or
    // fmt_writer.write_str()?; if you are only printing a string directly without formatting, but
    // still want escaping to be done.
    fmt_writer: FmtWriter<W>,
    state: State,
    preserve_whitespaces: bool,
    depth_stack: Vec<DepthData<'a>>,
    opt: Options,
}

impl<'a, W: Write> XmlWriter<'a, W> {
    /// Creates a new `XmlWriter`, writing data in the writer.
    #[inline]
    pub fn new(writer: W, opt: Options) -> Self {
        XmlWriter {
            fmt_writer: FmtWriter {
                writer,
                error_kind: None,
                escape: None,
                use_single_quote: opt.use_single_quote,
            },
            state: State::Empty,
            preserve_whitespaces: false,
            depth_stack: Vec::new(),
            opt,
        }
    }

    /// Writes an XML declaration.
    ///
    /// `<?xml version="1.0" encoding="UTF-8" standalone="no"?>`
    ///
    /// # Panics
    ///
    /// - When called twice.
    #[inline(never)]
    pub fn write_declaration(&mut self) -> io::Result<()> {
        if self.state != State::Empty {
            panic!("declaration was already written");
        }

        // Pretend that we are writing an element.
        self.state = State::Attributes;

        // <?xml version='1.0' encoding='UTF-8' standalone='yes'?>
        self.fmt_writer.writer.write_all(b"<?xml")?;
        // We don't use write_all() directly so that we get quoting handling for free.
        // However we can use the "raw" method here as we perfectly know there's no
        // escaping needed, albeit the performance impact would be almost inexistent if
        // we did use the regular method.
        self.write_attribute_raw("version", |w| w.write_all(b"1.0"))?;
        self.write_attribute_raw("encoding", |w| w.write_all(b"UTF-8"))?;
        self.write_attribute_raw("standalone", |w| w.write_all(b"no"))?;
        self.fmt_writer.writer.write_all(b"?>")?;

        self.state = State::Document;

        Ok(())
    }

    /// Writes a comment string.
    pub fn write_comment(&mut self, text: &str) -> io::Result<()> {
        self.write_comment_fmt(format_args!("{}", text))
    }

    /// Writes a formatted comment. Forbidden double hyphens will be escaped.
    #[inline(never)]
    pub fn write_comment_fmt(&mut self, fmt: fmt::Arguments) -> io::Result<()> {
        if self.state == State::Attributes {
            self.write_open_element()?;
        }

        if self.state != State::Empty {
            self.write_new_line()?;
        }

        self.write_node_indent()?;

        // <!--text-->
        self.fmt_writer.writer.write_all(b"<!--")?;
        self.fmt_writer.escape = Some(Escape::Comment);
        self.fmt_writer
            .write_fmt(fmt)
            .map_err(|_| self.fmt_writer.take_err())?;
        self.fmt_writer.writer.write_all(b"-->")?;

        if self.state == State::Attributes {
            self.depth_stack.push(DepthData {
                element_name: None,
                has_children: false,
            });
        }

        self.state = State::Document;

        Ok(())
    }

    /// Starts writing a new element.
    ///
    /// This method writes only the `<tag-name` part.
    #[inline(never)]
    pub fn start_element(&mut self, name: &'a str) -> io::Result<()> {
        if self.state == State::Attributes {
            self.write_open_element()?;
        }

        if self.state != State::Empty {
            self.write_new_line()?;
        }

        if !self.preserve_whitespaces {
            self.write_node_indent()?;
        }

        self.fmt_writer.writer.write_all(b"<")?;
        self.fmt_writer.writer.write_all(name.as_bytes())?;

        self.depth_stack.push(DepthData {
            element_name: Some(name),
            has_children: false,
        });

        self.state = State::Attributes;

        Ok(())
    }

    /// Writes an attribute.
    ///
    /// Any occurrence of `&<>"'` in the value will be escaped.
    ///
    /// # Panics
    ///
    /// - When called before `start_element()`.
    /// - When called after `close_element()`.
    ///
    /// # Example
    ///
    /// ```
    /// use xmlwriter::*;
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    ///     w.start_element("svg")?;
    ///     w.write_attribute("x", "5")?;
    ///     w.write_attribute("y", &5)?;
    ///     assert_eq!(std::str::from_utf8(w.end_document()?.as_slice())
    ///         .expect("xmlwriter should always produce valid UTF-8"),
    ///         "<svg x=\"5\" y=\"5\"/>\n",
    ///     );
    ///     Ok(())
    /// }
    /// ```
    pub fn write_attribute<V: Display + ?Sized>(
        &mut self,
        name: &str,
        value: &V,
    ) -> io::Result<()> {
        self.write_attribute_fmt(name, format_args!("{}", value))
    }

    /// Writes a formatted attribute value.
    ///
    /// Any occurrence of `&<>"'` in the value will be escaped.
    ///
    /// # Panics
    ///
    /// - When called before `start_element()`.
    /// - When called after `close_element()`.
    ///
    /// # Example
    ///
    /// ```
    /// use xmlwriter::*;
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    ///     w.start_element("rect")?;
    ///     w.write_attribute_fmt("fill", format_args!("url(#{})", "gradient"))?;
    ///     assert_eq!(std::str::from_utf8(w.end_document()?.as_slice())
    ///         .expect("xmlwriter should always produce valid UTF-8"),
    ///         "<rect fill=\"url(#gradient)\"/>\n"
    ///     );
    ///     Ok(())
    /// }
    /// ```
    #[inline(never)]
    pub fn write_attribute_fmt(&mut self, name: &str, fmt: fmt::Arguments) -> io::Result<()> {
        if self.state != State::Attributes {
            panic!("must be called after start_element()");
        }

        self.write_attribute_prefix(name)?;
        self.fmt_writer.escape = Some(Escape::AttributeValue);
        self.fmt_writer
            .write_fmt(fmt)
            .map_err(|_| self.fmt_writer.take_err())?;
        self.write_quote()
    }

    /// Writes a raw attribute value, without performing escaping.
    ///
    /// Closure provides a mutable reference to the writer.
    ///
    /// **Warning:** this method is an escape hatch for cases when you need to write
    /// a lot of data very fast, and as such does no validity checks whatsoever on the
    /// written value.
    ///
    /// # Panics
    ///
    /// - When called before `start_element()`.
    /// - When called after `close_element()`.
    ///
    /// # Example
    ///
    /// ```
    /// use xmlwriter::*;
    /// use std::io::{self, Write};
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    ///     w.start_element("path")?;
    ///     w.write_attribute_raw("d", |writer| writer.write_all(b"M 10 20 L 30 40"))?;
    ///     assert_eq!(std::str::from_utf8(w.end_document()?.as_slice())
    ///         .expect("xmlwriter should always produce valid UTF-8"),
    ///         "<path d=\"M 10 20 L 30 40\"/>\n"
    ///     );
    ///     Ok(())
    /// }
    /// ```
    #[inline(never)]
    pub fn write_attribute_raw<F>(&mut self, name: &str, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut W) -> io::Result<()>,
    {
        if self.state != State::Attributes {
            panic!("must be called after start_element()");
        }

        self.write_attribute_prefix(name)?;
        f(&mut self.fmt_writer.writer)?;
        self.write_quote()
    }

    #[inline(never)]
    fn write_attribute_prefix(&mut self, name: &str) -> io::Result<()> {
        if self.opt.attributes_indent == Indent::None {
            self.fmt_writer.writer.write_all(b" ")?;
        } else {
            self.fmt_writer.writer.write_all(b"\n")?;

            let depth = self.depth_stack.len();
            if depth > 0 {
                self.write_indent(depth - 1, self.opt.indent)?;
            }

            self.write_indent(1, self.opt.attributes_indent)?;
        }

        self.fmt_writer.writer.write_all(name.as_bytes())?;
        self.fmt_writer.writer.write_all(b"=")?;
        self.write_quote()
    }

    /// Sets the preserve whitespaces flag.
    ///
    /// - If set, text nodes will be written as is.
    /// - If not set, text nodes will be indented.
    ///
    /// Can be set at any moment.
    ///
    /// # Example
    ///
    /// ```
    /// use xmlwriter::*;
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    ///     w.start_element("html")?;
    ///     w.start_element("p")?;
    ///     w.write_text("text")?;
    ///     w.end_element()?;
    ///     w.start_element("p")?;
    ///     w.set_preserve_whitespaces(true);
    ///     w.write_text("text")?;
    ///     w.end_element()?;
    ///     w.set_preserve_whitespaces(false);
    ///     assert_eq!(std::str::from_utf8(w.end_document()?.as_slice())
    ///         .expect("xmlwriter should produce valid UTF-8"),
    /// "<html>
    ///     <p>
    ///         text
    ///     </p>
    ///     <p>text</p>
    /// </html>
    /// "
    ///     );
    ///     Ok(())
    /// }
    /// ```
    pub fn set_preserve_whitespaces(&mut self, preserve: bool) {
        self.preserve_whitespaces = preserve;
    }

    /// Writes a text node.
    ///
    /// See [`write_text_fmt()`] for details.
    ///
    /// [`write_text_fmt()`]: struct.XmlWriter.html#method.write_text_fmt
    pub fn write_text<T: Display + ?Sized>(&mut self, text: &T) -> io::Result<()> {
        self.write_text_fmt(format_args!("{}", text))
    }

    /// Writes a formatted text node.
    ///
    /// `><&` will be escaped.
    ///
    /// # Panics
    ///
    /// - When called not after `start_element()`.
    pub fn write_text_fmt(&mut self, fmt: fmt::Arguments) -> io::Result<()> {
        self.write_text_fmt_impl(fmt, false)
    }

    /// Writes text inside a `<![CDATA[ ... ]]>` node.
    ///
    /// # Panics
    ///
    /// - When called not after `start_element()`.
    /// - When the text contains the literal `]]>`.
    pub fn write_cdata_text(&mut self, text: &str) -> io::Result<()> {
        if text.contains("]]>") {
            panic!("CDATA text must not contain `]]>'");
        }
        self.write_text_fmt_impl(format_args!("{}", text), true)
    }

    #[inline(never)]
    fn write_text_fmt_impl(&mut self, fmt: fmt::Arguments, cdata: bool) -> io::Result<()> {
        if self.state == State::Empty || self.depth_stack.is_empty() {
            panic!("must be called after start_element()");
        }

        if self.state == State::Attributes {
            self.write_open_element()?;
        }

        if cdata && self.state != State::CData {
            self.fmt_writer.writer.write_all(b"<![CDATA[")?;
        }

        if self.state != State::Empty {
            self.write_new_line()?;
        }

        self.write_node_indent()?;

        self.fmt_writer.escape = Some(if cdata { Escape::CData } else { Escape::Text });
        self.fmt_writer
            .write_fmt(fmt)
            .map_err(|_| self.fmt_writer.take_err())?;

        if self.state == State::Attributes {
            self.depth_stack.push(DepthData {
                element_name: None,
                has_children: false,
            });
        }

        self.state = if cdata { State::CData } else { State::Document };

        Ok(())
    }

    /// Closes an open element.
    #[inline(never)]
    pub fn end_element(&mut self) -> io::Result<()> {
        if let Some(depth) = self.depth_stack.pop() {
            if depth.has_children {
                if !self.preserve_whitespaces {
                    self.write_new_line()?;
                    self.write_node_indent()?;
                }

                if self.state == State::CData {
                    self.fmt_writer.writer.write_all(b"]]>")?;
                }

                self.fmt_writer.writer.write_all(b"</")?;

                // Write the previous opening element name as closing element now.
                self.fmt_writer.writer.write_all(
                    depth
                        .element_name
                        .expect("did not have opening element name when closing element")
                        .as_bytes(),
                )?;

                self.fmt_writer.writer.write_all(b">")?;
            } else {
                self.fmt_writer.writer.write_all(b"/>")?;
            }
        }

        self.state = State::Document;

        Ok(())
    }

    /// Closes all open elements and returns back the writer.
    ///
    /// # Example
    ///
    /// ```
    /// use xmlwriter::*;
    /// use std::io;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    ///     w.start_element("svg")?;
    ///     w.start_element("g")?;
    ///     w.start_element("rect")?;
    ///     assert_eq!(std::str::from_utf8(w.end_document()?.as_slice())
    ///         .expect("xmlwriter should always produce valid UTF-8"),
    /// "<svg>
    ///     <g>
    ///         <rect/>
    ///     </g>
    /// </svg>
    /// "
    ///     );
    ///     Ok(())
    /// }
    /// ```
    pub fn end_document(mut self) -> io::Result<W> {
        while !self.depth_stack.is_empty() {
            self.end_element()?;
        }

        self.write_new_line()?;

        Ok(self.fmt_writer.writer)
    }

    #[inline]
    fn get_quote_char(&self) -> u8 {
        if self.opt.use_single_quote {
            b'\''
        } else {
            b'"'
        }
    }

    // Writes quote unescaped, so only use when appropriate.
    #[inline]
    fn write_quote(&mut self) -> io::Result<()> {
        self.fmt_writer.writer.write_all(&[self.get_quote_char()])
    }

    // Writes the end of the current opening element, so `>`.
    fn write_open_element(&mut self) -> io::Result<()> {
        if let Some(depth) = self.depth_stack.last_mut() {
            depth.has_children = true;
            self.fmt_writer.writer.write_all(b">")?;

            self.state = State::Document;
        }
        Ok(())
    }

    fn write_node_indent(&mut self) -> io::Result<()> {
        self.write_indent(self.depth_stack.len(), self.opt.indent)
    }

    fn write_indent(&mut self, depth: usize, indent: Indent) -> io::Result<()> {
        if indent == Indent::None || self.preserve_whitespaces {
            return Ok(());
        }

        for _ in 0..depth {
            match indent {
                Indent::None => {}
                Indent::Spaces(n) => {
                    for _ in 0..n {
                        self.fmt_writer.writer.write_all(b" ")?;
                    }
                }
                Indent::Tabs => self.fmt_writer.writer.write_all(b"\t")?,
            }
        }
        Ok(())
    }

    fn write_new_line(&mut self) -> io::Result<()> {
        if self.opt.indent != Indent::None && !self.preserve_whitespaces {
            self.fmt_writer.writer.write_all(b"\n")?;
        }
        Ok(())
    }
}
