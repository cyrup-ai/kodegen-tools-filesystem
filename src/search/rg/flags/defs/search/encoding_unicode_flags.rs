//! Encoding and Unicode handling flags.

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::{EncodingMode, LowArgs},
};

use super::super::CompletionType;

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

/// -E/--encoding
#[derive(Debug)]
pub(super) struct Encoding;

impl Flag for Encoding {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'E')
    }
    fn name_long(&self) -> &'static str {
        "encoding"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-encoding")
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("ENCODING")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Specify the text encoding of files to search."
    }
    fn doc_long(&self) -> &'static str {
        r"
Specify the text encoding that ripgrep will use on all files searched. The
default value is \fBauto\fP, which will cause ripgrep to do a best effort
automatic detection of encoding on a per-file basis. Automatic detection in
this case only applies to files that begin with a UTF-8 or UTF-16 byte-order
mark (BOM). No other automatic detection is performed. One can also specify
\fBnone\fP which will then completely disable BOM sniffing and always result
in searching the raw bytes, including a BOM if it's present, regardless of its
encoding.
.sp
Other supported values can be found in the list of labels here:
\fIhttps://encoding.spec.whatwg.org/#concept-encoding-get\fP.
.sp
For more details on encoding and how ripgrep deals with it, see \fBGUIDE.md\fP.
.sp
The encoding detection that ripgrep uses can be reverted to its automatic mode
via the \flag-negate{encoding} flag.
"
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Encoding
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let value = match v {
            FlagValue::Value(v) => v,
            FlagValue::Switch(true) => {
                unreachable!("--encoding must accept a value")
            }
            FlagValue::Switch(false) => {
                args.encoding = EncodingMode::Auto;
                return Ok(());
            }
        };
        let label = super::super::convert::str(&value)?;
        args.encoding = match label {
            "auto" => EncodingMode::Auto,
            "none" => EncodingMode::Disabled,
            _ => EncodingMode::Some(grep::searcher::Encoding::new(label)?),
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_encoding() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(EncodingMode::Auto, args.encoding);

    let args = parse_low_raw(["--encoding", "auto"]).expect("Test parsing should succeed");
    assert_eq!(EncodingMode::Auto, args.encoding);

    let args = parse_low_raw(["--encoding", "none"]).expect("Test parsing should succeed");
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["--encoding=none"]).expect("Test parsing should succeed");
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-E", "none"]).expect("Test parsing should succeed");
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-Enone"]).expect("Test parsing should succeed");
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-E", "none", "--no-encoding"]).expect("Test parsing should succeed");
    assert_eq!(EncodingMode::Auto, args.encoding);

    let args = parse_low_raw(["--no-encoding", "-E", "none"]).expect("Test parsing should succeed");
    assert_eq!(EncodingMode::Disabled, args.encoding);

    let args = parse_low_raw(["-E", "utf-16"]).expect("Test parsing should succeed");
    let enc = grep::searcher::Encoding::new("utf-16").expect("Test parsing should succeed");
    assert_eq!(EncodingMode::Some(enc), args.encoding);

    let args = parse_low_raw(["-E", "utf-16", "--no-encoding"]).expect("Test parsing should succeed");
    assert_eq!(EncodingMode::Auto, args.encoding);

    let result = parse_low_raw(["-E", "foo"]);
    assert!(result.is_err(), "{result:?}");
}

/// --no-unicode
#[derive(Debug)]
pub(super) struct NoUnicode;

impl Flag for NoUnicode {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-unicode"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("unicode")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Disable Unicode mode."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag disables Unicode mode for all patterns given to ripgrep.
.sp
By default, ripgrep will enable "Unicode mode" in all of its regexes. This has
a number of consequences:
.sp
.IP \(bu 3n
\fB.\fP will only match valid UTF-8 encoded Unicode scalar values.
.sp
.IP \(bu 3n
Classes like \fB\\w\fP, \fB\\s\fP, \fB\\d\fP are all Unicode aware and much
bigger than their ASCII only versions.
.sp
.IP \(bu 3n
Case insensitive matching will use Unicode case folding.
.sp
.IP \(bu 3n
A large array of classes like \fB\\p{Emoji}\fP are available. (Although the
specific set of classes available varies based on the regex engine. In general,
the default regex engine has more classes available to it.)
.sp
.IP \(bu 3n
Word boundaries (\fB\\b\fP and \fB\\B\fP) use the Unicode definition of a word
character.
.PP
In some cases it can be desirable to turn these things off. This flag will do
exactly that. For example, Unicode mode can sometimes have a negative impact
on performance, especially when things like \fB\\w\fP are used frequently
(including via bounded repetitions like \fB\\w{100}\fP) when only their ASCII
interpretation is needed.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_unicode = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_unicode() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-unicode"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_unicode);

    let args = parse_low_raw(["--no-unicode", "--unicode"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-unicode", "--pcre2-unicode"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-pcre2-unicode", "--unicode"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_unicode);
}

/// --no-pcre2-unicode
#[derive(Debug)]
pub(super) struct NoPcre2Unicode;

impl Flag for NoPcre2Unicode {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "no-pcre2-unicode"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("pcre2-unicode")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"(DEPRECATED) Disable Unicode mode for PCRE2."
    }
    fn doc_long(&self) -> &'static str {
        r"
DEPRECATED. Use \flag{no-unicode} instead.
.sp
Note that Unicode mode is enabled by default.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.no_unicode = v.unwrap_switch();
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_no_pcre2_unicode() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(false, args.no_unicode);

    let args = parse_low_raw(["--no-pcre2-unicode"]).expect("Test parsing should succeed");
    assert_eq!(true, args.no_unicode);

    let args =
        parse_low_raw(["--no-pcre2-unicode", "--pcre2-unicode"]).expect("Test parsing should succeed");
    assert_eq!(false, args.no_unicode);
}
