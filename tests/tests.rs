use std::{
    io::{self, Write},
    str::from_utf8,
};
use xmlwriter::{Options, XmlWriter};

macro_rules! text_eq {
    ($result:expr, $expected:expr) => {
        assert_eq!(
            from_utf8($result.as_slice()).expect("XmlWriter should produce valid UTF8"),
            $expected,
        )
    };
}

#[test]
fn write_element_01() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("svg")?;
    w.end_element()?;
    text_eq!(w.end_document()?, "<svg/>\n");
    Ok(())
}

#[test]
fn write_element_02() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("svg")?;
    w.start_element("rect")?;
    w.end_element()?;
    w.end_element()?;
    text_eq!(
        w.end_document()?,
        r#"<svg>
    <rect/>
</svg>
"#
    );
    Ok(())
}

#[test]
fn write_element_03() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("svg")?;
    w.end_element()?;
    w.end_element()?; // Should not panic.
    text_eq!(w.end_document()?, "<svg/>\n");
    Ok(())
}

#[test]
fn write_element_05() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("svg")?;
    // end_document() will call `close_element` automatically.
    text_eq!(w.end_document()?, "<svg/>\n");
    Ok(())
}

#[test]
fn write_element_06() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("svg")?;
    w.start_element("rect")?;
    w.start_element("rect")?;
    w.start_element("rect")?;
    w.start_element("rect")?;
    w.start_element("rect")?;
    text_eq!(
        w.end_document()?,
        r#"<svg>
    <rect>
        <rect>
            <rect>
                <rect>
                    <rect/>
                </rect>
            </rect>
        </rect>
    </rect>
</svg>
"#
    );
    Ok(())
}

#[test]
#[should_panic(expected = "must be called after start_element()")]
fn write_attribute_01() {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    // must be used only after write_element
    w.write_attribute("id", "q")
        .expect("no IO error since we're supposed to panic first");
}

#[test]
fn write_attribute_02() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("svg")?;
    w.write_attribute("id", "q")?;
    w.end_element()?;
    text_eq!(w.end_document()?, "<svg id=\"q\"/>\n");
    Ok(())
}

#[test]
fn write_attribute_03() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("svg")?;
    w.write_attribute("id", "\"")?;
    w.end_element()?;
    text_eq!(w.end_document()?, "<svg id=\"&quot;\"/>\n");
    Ok(())
}

#[test]
fn write_attribute_04() -> io::Result<()> {
    let opt = Options {
        use_single_quote: true,
        ..Options::default()
    };

    let mut w = XmlWriter::new(Vec::<u8>::new(), opt);
    w.start_element("svg")?;
    w.write_attribute("id", "'")?;
    w.end_element()?;
    text_eq!(w.end_document()?, "<svg id='&apos;'/>\n");
    Ok(())
}

#[test]
fn write_attribute_05() -> io::Result<()> {
    let opt = Options {
        use_single_quote: true,
        ..Options::default()
    };

    let mut w = XmlWriter::new(Vec::<u8>::new(), opt);
    w.start_element("svg")?;
    w.write_attribute("id", "'''\"'\"\"'")?;
    w.end_element()?;
    text_eq!(
        w.end_document()?,
        // We should only escape single quotes in attribute values when we're
        // using single quotes around them too. In that case double quotes
        // should not be escaped.
        "<svg id='&apos;&apos;&apos;\"&apos;\"\"&apos;'/>\n"
    );
    Ok(())
}

// Same as write_attribute_05(), but to make sure single quotes aren't escaped
// when using double quotes.
#[test]
fn write_attribute_06() -> io::Result<()> {
    let opt = Options {
        use_single_quote: false,
        ..Options::default()
    };

    let mut w = XmlWriter::new(Vec::<u8>::new(), opt);
    w.start_element("svg")?;
    w.write_attribute("id", "'''\"'\"\"'")?;
    w.end_element()?;
    text_eq!(
        w.end_document()?,
        // We should only escape single quotes in attribute values when we're
        // using single quotes around them too. In that case double quotes
        // should not be escaped.
        "<svg id=\"'''&quot;'&quot;&quot;'\"/>\n"
    );
    Ok(())
}

#[test]
fn write_attribute_07() -> io::Result<()> {
    let opt = Options {
        use_single_quote: true,
        ..Options::default()
    };

    let mut w = XmlWriter::new(Vec::<u8>::new(), opt);
    w.start_element("svg")?;
    w.write_attribute("id", "'text'")?;
    w.end_element()?;
    text_eq!(w.end_document()?, "<svg id='&apos;text&apos;'/>\n");
    Ok(())
}

#[test]
fn write_attribute_08() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("svg")?;
    // TODO: looks we need specialization to remove &
    w.write_attribute("x", &5)?;
    w.end_element()?;
    text_eq!(w.end_document()?, "<svg x=\"5\"/>\n");
    Ok(())
}

#[test]
fn write_attribute_09() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("foo")?;
    w.write_attribute_raw("x", |writer| writer.write_all(br#"&&"''"<>><>"#))?;
    w.end_element()?;
    text_eq!(
        w.end_document()?,
        // It is invalid XML if you do that, but it'd be your fault then :)
        r#"<foo x="&&"''"<>><>"/>
"#
    );
    Ok(())
}

#[test]
fn write_declaration_01() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.write_declaration()?;
    text_eq!(
        w.end_document()?,
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n"
    );
    Ok(())
}

#[test]
#[should_panic(expected = "declaration was already written")]
fn write_declaration_02() {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.write_declaration()
        .expect("we should only be panicking on the next line!");
    w.write_declaration()
        .expect("we'll panic before even returning a Result"); // declaration must be written once
}

#[test]
// declaration was already written
#[should_panic(expected = "declaration was already written")]
fn write_declaration_03() {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.write_comment("test").expect("no error expected here!");
    w.write_declaration()
        .expect("we'll panic before even returning a Result"); // declaration must be written first
}

#[test]
fn write_single_quote_01() -> io::Result<()> {
    let opt = Options {
        use_single_quote: true,
        ..Options::default()
    };

    let mut w = XmlWriter::new(Vec::<u8>::new(), opt);
    w.write_declaration()?;
    text_eq!(
        w.end_document()?,
        "<?xml version='1.0' encoding='UTF-8' standalone='no'?>\n"
    );
    Ok(())
}

#[test]
fn write_single_quote_02() -> io::Result<()> {
    let opt = Options {
        use_single_quote: true,
        ..Options::default()
    };

    let mut w = XmlWriter::new(Vec::<u8>::new(), opt);
    w.start_element("p")?;
    w.write_attribute("a", "b")?;
    w.end_element()?;
    text_eq!(w.end_document()?, "<p a='b'/>\n");
    Ok(())
}

#[test]
fn write_comment_01() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.write_comment("test")?;
    w.start_element("svg")?;
    text_eq!(
        w.end_document()?,
        r#"<!--test-->
<svg/>
"#
    );
    Ok(())
}

#[test]
fn write_comment_02() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("svg")?;
    w.write_comment("test")?;
    text_eq!(
        w.end_document()?,
        r#"<svg>
    <!--test-->
</svg>
"#
    );
    Ok(())
}

#[test]
fn write_comment_03() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.write_comment("test")?;
    w.start_element("svg")?;
    w.write_comment("test")?;
    text_eq!(
        w.end_document()?,
        r#"<!--test-->
<svg>
    <!--test-->
</svg>
"#
    );
    Ok(())
}

#[test]
fn write_comment_04() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.write_comment("test")?;
    w.start_element("svg")?;
    w.start_element("rect")?;
    w.write_comment("test")?;
    text_eq!(
        w.end_document()?,
        r#"<!--test-->
<svg>
    <rect>
        <!--test-->
    </rect>
</svg>
"#
    );
    Ok(())
}

#[test]
fn write_comment_05() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.write_comment("test")?;
    w.start_element("svg")?;
    w.write_comment("test")?;
    w.start_element("rect")?;
    w.end_element()?;
    text_eq!(
        w.end_document()?,
        r#"<!--test-->
<svg>
    <!--test-->
    <rect/>
</svg>
"#
    );
    Ok(())
}

#[test]
fn write_comment_06() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.write_comment("test")?;
    w.start_element("svg")?;
    w.start_element("rect")?;
    w.end_element()?;
    w.write_comment("test")?;
    text_eq!(
        w.end_document()?,
        r#"<!--test-->
<svg>
    <rect/>
    <!--test-->
</svg>
"#
    );
    Ok(())
}

#[test]
fn write_comment_07() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("svg")?;
    w.end_element()?;
    w.write_comment("test")?;
    text_eq!(
        w.end_document()?,
        r#"<svg/>
<!--test-->
"#
    );
    Ok(())
}

#[test]
fn write_comment_08() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.write_comment("test")?;
    w.write_comment("test")?;
    w.write_comment("test")?;
    text_eq!(
        w.end_document()?,
        r#"<!--test-->
<!--test-->
<!--test-->
"#
    );
    Ok(())
}

#[test]
#[should_panic(expected = "must be called after start_element()")]
fn write_text_01() {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.write_text("text")
        .expect("should panic before giving us a Result"); // Should be called after start_element()
}

#[test]
#[should_panic(expected = "must be called after start_element()")]
fn write_text_03() {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("p").expect("should not fail");
    w.end_element().expect("should not fail");
    w.write_text("text")
        .expect("should panic before giving us a Result"); // Should be called after start_element()
}

#[test]
fn write_text_04() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("p")?;
    w.write_text("text")?;
    w.write_text("text")?;
    text_eq!(
        w.end_document()?,
        r#"<p>
    text
    text
</p>
"#
    );
    Ok(())
}

#[test]
fn write_text_05() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("p")?;
    w.write_text("text")?;
    text_eq!(
        w.end_document()?,
        r#"<p>
    text
</p>
"#
    );
    Ok(())
}

#[test]
fn write_text_06() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("p")?;
    w.write_text("text")?;
    w.start_element("p")?;
    w.write_text("text")?;
    text_eq!(
        w.end_document()?,
        r#"<p>
    text
    <p>
        text
    </p>
</p>
"#
    );
    Ok(())
}

#[test]
fn write_text_07() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("div")?;
    w.start_element("p")?;
    w.write_text("text")?;
    w.start_element("p")?;
    w.write_text("text")?;
    text_eq!(
        w.end_document()?,
        r#"<div>
    <p>
        text
        <p>
            text
        </p>
    </p>
</div>
"#
    );
    Ok(())
}

#[test]
fn write_text_08() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("p")?;
    w.write_text("<")?;
    text_eq!(
        w.end_document()?,
        r#"<p>
    &lt;
</p>
"#
    );
    Ok(())
}

#[test]
fn write_text_09() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("p")?;
    w.write_text("<&>")?;
    text_eq!(
        w.end_document()?,
        r#"<p>
    &lt;&amp;&gt;
</p>
"#
    );
    Ok(())
}

#[test]
fn write_text_10() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("p")?;
    w.write_text("&lt;")?;
    text_eq!(
        w.end_document()?,
        r#"<p>
    &amp;lt;
</p>
"#
    );
    Ok(())
}

#[test]
fn write_text_11() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("p")?;
    w.write_text("text")?;
    w.start_element("p")?;
    w.end_element()?;
    w.write_text("text")?;
    text_eq!(
        w.end_document()?,
        r#"<p>
    text
    <p/>
    text
</p>
"#
    );
    Ok(())
}

#[test]
fn write_text_12() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("p")?;
    w.write_attribute("att", "fooo&bar&&&&amp;&amp;&baz&&&")?;
    w.write_text("fooo&bar&&&&amp;&amp;&baz&&&")?;
    text_eq!(
        w.end_document()?,
        r#"<p att="fooo&amp;bar&amp;&amp;&amp;&amp;amp;&amp;amp;&amp;baz&amp;&amp;&amp;">
    fooo&amp;bar&amp;&amp;&amp;&amp;amp;&amp;amp;&amp;baz&amp;&amp;&amp;
</p>
"#
    );
    Ok(())
}

#[test]
fn write_text_cdata() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("script")?;
    w.write_cdata_text("function cmp(a,b) { return (a<b)?-1:(a>b)?1:0; }")?;
    text_eq!(
        w.end_document()?,
        "<script><![CDATA[
    function cmp(a,b) { return (a<b)?-1:(a>b)?1:0; }
]]></script>
"
    );
    Ok(())
}

#[test]
fn write_preserve_text_01() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.set_preserve_whitespaces(true);
    w.start_element("p")?;
    w.write_text("text")?;
    w.start_element("p")?;
    w.end_element()?;
    w.write_text("text")?;
    text_eq!(w.end_document()?, "<p>text<p/>text</p>");
    Ok(())
}

#[test]
fn write_preserve_text_02() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("p")?;
    w.start_element("p")?;
    w.set_preserve_whitespaces(true);
    w.write_text("text")?;
    w.start_element("p")?;
    w.end_element()?;
    w.write_text("text")?;
    w.end_element()?;
    w.set_preserve_whitespaces(false);
    text_eq!(
        w.end_document()?,
        r#"<p>
    <p>text<p/>text</p>
</p>
"#
    );
    Ok(())
}

#[test]
fn attrs_indent_01() -> io::Result<()> {
    let opt = Options {
        attributes_indent: xmlwriter::Indent::Spaces(2),
        ..Options::default()
    };

    let mut w = XmlWriter::new(Vec::<u8>::new(), opt);
    w.start_element("rect")?;
    w.write_attribute("x", "5")?;
    w.start_element("rect")?;
    w.write_attribute("x", "10")?;
    w.write_attribute("y", "15")?;
    text_eq!(
        w.end_document()?,
        r#"<rect
  x="5">
    <rect
      x="10"
      y="15"/>
</rect>
"#
    );
    Ok(())
}

// At some point I had used split_at() with a byte index but that does not work for multi-bytes
// characters, so let's that to make sure it isn't reintroduced.
#[test]
fn multibytes_escaping() -> io::Result<()> {
    let mut w = XmlWriter::new(Vec::<u8>::new(), Options::default());
    w.start_element("test")?;
    w.write_attribute("foo", "aaa&bbb<ccc•&•>&•")?;
    w.write_attribute_fmt("bar", format_args!("aaa&bbb<ccc{}&•>&•", '•'))?;
    w.write_text("aaa&bbb<ccc•&•>&•")?;
    w.write_text_fmt(format_args!("aaa&bbb<ccc{}&•>&•", '•'))?;

    text_eq!(
        w.end_document()?,
        r#"<test foo="aaa&amp;bbb&lt;ccc•&amp;•&gt;&amp;•" bar="aaa&amp;bbb&lt;ccc•&amp;•&gt;&amp;•">
    aaa&amp;bbb&lt;ccc•&amp;•&gt;&amp;•
    aaa&amp;bbb&lt;ccc•&amp;•&gt;&amp;•
</test>
"#
    );
    Ok(())
}

#[test]
fn disabled_self_close() -> io::Result<()> {
    let opts = Options {
        enable_self_closing: false,
        ..Options::default()
    };
    let mut w = XmlWriter::new(Vec::<u8>::new(), opts);
    w.start_element("empty1")?;
    w.end_element()?;
    w.start_element("wrapper")?;
    w.start_element("empty2")?;
    w.end_element()?;
    w.end_element()?;

    text_eq!(
        w.end_document()?,
        r#"<empty1>
</empty1>
<wrapper>
    <empty2>
    </empty2>
</wrapper>
"#
    );

    Ok(())
}
