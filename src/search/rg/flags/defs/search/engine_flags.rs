//! Regex engine selection flags.

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::{Engine, LowArgs},
};

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

/// --auto-hybrid-regex
#[derive(Debug)]
pub(super) struct AutoHybridRegex;

impl Flag for AutoHybridRegex {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "auto-hybrid-regex"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-auto-hybrid-regex")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        "(DEPRECATED) Use PCRE2 if appropriate."
    }
    fn doc_long(&self) -> &'static str {
        r"
DEPRECATED. Use \flag{engine} instead.
.sp
When this flag is used, ripgrep will dynamically choose between supported regex
engines depending on the features used in a pattern. When ripgrep chooses a
regex engine, it applies that choice for every regex provided to ripgrep (e.g.,
via multiple \flag{regexp} or \flag{file} flags).
.sp
As an example of how this flag might behave, ripgrep will attempt to use
its default finite automata based regex engine whenever the pattern can be
successfully compiled with that regex engine. If PCRE2 is enabled and if the
pattern given could not be compiled with the default regex engine, then PCRE2
will be automatically used for searching. If PCRE2 isn't available, then this
flag has no effect because there is only one regex engine to choose from.
.sp
In the future, ripgrep may adjust its heuristics for how it decides which
regex engine to use. In general, the heuristics will be limited to a static
analysis of the patterns, and not to any specific runtime behavior observed
while searching files.
.sp
The primary downside of using this flag is that it may not always be obvious
which regex engine ripgrep uses, and thus, the match semantics or performance
profile of ripgrep may subtly and unexpectedly change. However, in many cases,
all regex engines will agree on what constitutes a match and it can be nice
to transparently support more advanced regex features like look-around and
backreferences without explicitly needing to enable them.
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let mode = if v.unwrap_switch() {
            Engine::Auto
        } else {
            Engine::Default
        };
        args.engine = mode;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_auto_hybrid_regex() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Engine::Default, args.engine);

    let args = parse_low_raw(["--auto-hybrid-regex"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--no-auto-hybrid-regex"])
            .expect("Test parsing should succeed");
    assert_eq!(Engine::Default, args.engine);

    let args =
        parse_low_raw(["--no-auto-hybrid-regex", "--auto-hybrid-regex"])
            .expect("Test parsing should succeed");
    assert_eq!(Engine::Auto, args.engine);

    let args = parse_low_raw(["--auto-hybrid-regex", "-P"]).expect("Test parsing should succeed");
    assert_eq!(Engine::PCRE2, args.engine);

    let args = parse_low_raw(["-P", "--auto-hybrid-regex"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Auto, args.engine);

    let args =
        parse_low_raw(["--engine=auto", "--auto-hybrid-regex"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Auto, args.engine);

    let args =
        parse_low_raw(["--engine=default", "--auto-hybrid-regex"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=default"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Default, args.engine);
}

/// --engine
#[derive(Debug)]
pub(super) struct EngineFlag;

impl Flag for EngineFlag {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "engine"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("ENGINE")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Specify which regex engine to use."
    }
    fn doc_long(&self) -> &'static str {
        r"
Specify which regular expression engine to use. When you choose a regex engine,
it applies that choice for every regex provided to ripgrep (e.g., via multiple
\flag{regexp} or \flag{file} flags).
.sp
Accepted values are \fBdefault\fP, \fBpcre2\fP, or \fBauto\fP.
.sp
The default value is \fBdefault\fP, which is usually the fastest and should be
good for most use cases. The \fBpcre2\fP engine is generally useful when you
want to use features such as look-around or backreferences. \fBauto\fP will
dynamically choose between supported regex engines depending on the features
used in a pattern on a best effort basis.
.sp
Note that the \fBpcre2\fP engine is an optional ripgrep feature. If PCRE2
wasn't included in your build of ripgrep, then using this flag will result in
ripgrep printing an error message and exiting.
.sp
This overrides previous uses of the \flag{pcre2} and \flag{auto-hybrid-regex}
flags.
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &["default", "pcre2", "auto"]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        let string = super::super::convert::str(&v)?;
        args.engine = match string {
            "default" => Engine::Default,
            "pcre2" => Engine::PCRE2,
            "auto" => Engine::Auto,
            _ => anyhow::bail!("unrecognized regex engine '{string}'"),
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_engine() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Engine::Default, args.engine);

    let args = parse_low_raw(["--engine", "pcre2"]).expect("Test parsing should succeed");
    assert_eq!(Engine::PCRE2, args.engine);

    let args = parse_low_raw(["--engine=pcre2"]).expect("Test parsing should succeed");
    assert_eq!(Engine::PCRE2, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=pcre2"]).expect("Test parsing should succeed");
    assert_eq!(Engine::PCRE2, args.engine);

    let args =
        parse_low_raw(["--engine=pcre2", "--auto-hybrid-regex"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=auto"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Auto, args.engine);

    let args =
        parse_low_raw(["--auto-hybrid-regex", "--engine=default"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Default, args.engine);

    let args =
        parse_low_raw(["--engine=pcre2", "--no-auto-hybrid-regex"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Default, args.engine);
}

/// -P/--pcre2
#[derive(Debug)]
pub(super) struct PCRE2;

impl Flag for PCRE2 {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'P')
    }
    fn name_long(&self) -> &'static str {
        "pcre2"
    }
    fn name_negated(&self) -> Option<&'static str> {
        Some("no-pcre2")
    }
    fn doc_category(&self) -> Category {
        Category::Search
    }
    fn doc_short(&self) -> &'static str {
        r"Enable PCRE2 matching."
    }
    fn doc_long(&self) -> &'static str {
        r"
When this flag is present, ripgrep will use the PCRE2 regex engine instead of
its default regex engine.
.sp
This is generally useful when you want to use features such as look-around
or backreferences.
.sp
Using this flag is the same as passing \fB\-\-engine=pcre2\fP. Users may
instead elect to use \fB\-\-engine=auto\fP to ask ripgrep to automatically
select the right regex engine based on the patterns given. This flag and the
\flag{engine} flag override one another.
.sp
Note that PCRE2 is an optional ripgrep feature. If PCRE2 wasn't included in
your build of ripgrep, then using this flag will result in ripgrep printing
an error message and exiting. PCRE2 may also have worse user experience in
some cases, since it has fewer introspection APIs than ripgrep's default
regex engine. For example, if you use a \fB\\n\fP in a PCRE2 regex without
the \flag{multiline} flag, then ripgrep will silently fail to match anything
instead of reporting an error immediately (like it does with the default regex
engine).
"
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.engine = if v.unwrap_switch() {
            Engine::PCRE2
        } else {
            Engine::Default
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_pcre2() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(Engine::Default, args.engine);

    let args = parse_low_raw(["--pcre2"]).expect("Test parsing should succeed");
    assert_eq!(Engine::PCRE2, args.engine);

    let args = parse_low_raw(["-P"]).expect("Test parsing should succeed");
    assert_eq!(Engine::PCRE2, args.engine);

    let args = parse_low_raw(["-P", "--no-pcre2"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Default, args.engine);

    let args = parse_low_raw(["--engine=auto", "-P", "--no-pcre2"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Default, args.engine);

    let args = parse_low_raw(["-P", "--engine=auto"]).expect("Test parsing should succeed");
    assert_eq!(Engine::Auto, args.engine);
}
