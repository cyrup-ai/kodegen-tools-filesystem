//! Color and hyperlink output flags.

use std::{path::PathBuf, sync::LazyLock};
use anyhow::Context as AnyhowContext;

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::{ColorChoice, LowArgs},
};

#[cfg(test)]
use crate::search::rg::flags::parse::parse_low_raw;

use super::super::{CompletionType, convert};

/// --color
#[derive(Debug)]
pub(in crate::search::rg::flags) struct Color;

impl Flag for Color {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "color"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("WHEN")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "When to use color."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag controls when to use colors. The default setting is \fBauto\fP, which
means ripgrep will try to guess when to use colors. For example, if ripgrep is
printing to a tty, then it will use colors, but if it is redirected to a file
or a pipe, then it will suppress color output.
.sp
ripgrep will suppress color output by default in some other circumstances as
well. These include, but are not limited to:
.sp
.IP \(bu 3n
When the \fBTERM\fP environment variable is not set or set to \fBdumb\fP.
.sp
.IP \(bu 3n
When the \fBNO_COLOR\fP environment variable is set (regardless of value).
.sp
.IP \(bu 3n
When flags that imply no use for colors are given. For example,
\flag{vimgrep} and \flag{json}.
.
.PP
The possible values for this flag are:
.sp
.IP \fBnever\fP 10n
Colors will never be used.
.sp
.IP \fBauto\fP 10n
The default. ripgrep tries to be smart.
.sp
.IP \fBalways\fP 10n
Colors will always be used regardless of where output is sent.
.sp
.IP \fBansi\fP 10n
Like 'always', but emits ANSI escapes (even in a Windows console).
.
.PP
This flag also controls whether hyperlinks are emitted. For example, when
a hyperlink format is specified, hyperlinks won't be used when color is
suppressed. If one wants to emit hyperlinks but no colors, then one must use
the \flag{colors} flag to manually set all color styles to \fBnone\fP:
.sp
.EX
    \-\-colors 'path:none' \\
    \-\-colors 'line:none' \\
    \-\-colors 'column:none' \\
    \-\-colors 'match:none' \\
    \-\-colors 'highlight:none'
.EE
.sp
"
    }
    fn doc_choices(&self) -> &'static [&'static str] {
        &["never", "auto"]
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        args.color = match convert::str(&v.unwrap_value())? {
            "never" => ColorChoice::Never,
            "auto" => ColorChoice::Auto,
            unk => anyhow::bail!("choice '{unk}' is unrecognized"),
        };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_color() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Auto, args.color);

    let args = parse_low_raw(["--color", "never"]).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Never, args.color);

    let args = parse_low_raw(["--color", "auto"]).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Auto, args.color);

    let args = parse_low_raw(["--color=never"]).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Never, args.color);

    let args =
        parse_low_raw(["--color", "auto", "--color", "never"]).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Never, args.color);

    let args =
        parse_low_raw(["--color", "never", "--color", "auto"]).expect("Test parsing should succeed");
    assert_eq!(ColorChoice::Auto, args.color);

    let result = parse_low_raw(["--color", "foofoo"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--color", "always"]);
    assert!(result.is_err(), "{result:?}");

    let result = parse_low_raw(["--color", "ansi"]);
    assert!(result.is_err(), "{result:?}");
}

/// --colors
#[derive(Debug)]
pub(in crate::search::rg::flags) struct Colors;

impl Flag for Colors {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "colors"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("COLOR_SPEC")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        "Configure color settings and styles."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag specifies color settings for use in the output. This flag may be
provided multiple times. Settings are applied iteratively. Pre-existing color
labels are limited to one of eight choices: \fBred\fP, \fBblue\fP, \fBgreen\fP,
\fBcyan\fP, \fBmagenta\fP, \fByellow\fP, \fBwhite\fP and \fBblack\fP. Styles
are limited to \fBnobold\fP, \fBbold\fP, \fBnointense\fP, \fBintense\fP,
\fBnounderline\fP, \fBunderline\fP, \fBnoitalic\fP or \fBitalic\fP.
.sp
The format of the flag is
\fB{\fP\fItype\fP\fB}:{\fP\fIattribute\fP\fB}:{\fP\fIvalue\fP\fB}\fP.
\fItype\fP should be one of \fBpath\fP, \fBline\fP, \fBcolumn\fP,
\fBhighlight\fP or \fBmatch\fP. \fIattribute\fP can be \fBfg\fP, \fBbg\fP or
\fBstyle\fP. \fIvalue\fP is either a color (for \fBfg\fP and \fBbg\fP) or a
text style. A special format, \fB{\fP\fItype\fP\fB}:none\fP, will clear all
color settings for \fItype\fP.
.sp
For example, the following command will change the match color to magenta and
the background color for line numbers to yellow:
.sp
.EX
    rg \-\-colors 'match:fg:magenta' \-\-colors 'line:bg:yellow'
.EE
.sp
Another example, the following command will "highlight" the non-matching text
in matching lines:
.sp
.EX
    rg \-\-colors 'highlight:bg:yellow' \-\-colors 'highlight:fg:black'
.EE
.sp
The "highlight" color type is particularly useful for contrasting matching
lines with surrounding context printed by the \flag{before-context},
\flag{after-context}, \flag{context} or \flag{passthru} flags.
.sp
Extended colors can be used for \fIvalue\fP when the tty supports ANSI color
sequences. These are specified as either \fIx\fP (256-color) or
.IB x , x , x
(24-bit truecolor) where \fIx\fP is a number between \fB0\fP and \fB255\fP
inclusive. \fIx\fP may be given as a normal decimal number or a hexadecimal
number, which is prefixed by \fB0x\fP.
.sp
For example, the following command will change the match background color to
that represented by the rgb value (0,128,255):
.sp
.EX
    rg \-\-colors 'match:bg:0,128,255'
.EE
.sp
or, equivalently,
.sp
.EX
    rg \-\-colors 'match:bg:0x0,0x80,0xFF'
.EE
.sp
Note that the \fBintense\fP and \fBnointense\fP styles will have no effect when
used alongside these extended color codes.
"#
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        let v = convert::str(&v)?;
        args.colors.push(v.parse()?);
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_colors() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert!(args.colors.is_empty());

    let args = parse_low_raw(["--colors", "match:fg:magenta"]).expect("Test parsing should succeed");
    assert_eq!(args.colors, vec!["match:fg:magenta".parse().expect("Test parsing should succeed")]);

    let args = parse_low_raw([
        "--colors",
        "match:fg:magenta",
        "--colors",
        "line:bg:yellow",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(
        args.colors,
        vec![
            "match:fg:magenta".parse().expect("Test parsing should succeed"),
            "line:bg:yellow".parse().expect("Test parsing should succeed")
        ]
    );

    let args = parse_low_raw(["--colors", "highlight:bg:240"]).expect("Test parsing should succeed");
    assert_eq!(args.colors, vec!["highlight:bg:240".parse().expect("Test parsing should succeed")]);

    let args = parse_low_raw([
        "--colors",
        "match:fg:magenta",
        "--colors",
        "highlight:bg:blue",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(
        args.colors,
        vec![
            "match:fg:magenta".parse().expect("Test parsing should succeed"),
            "highlight:bg:blue".parse().expect("Test parsing should succeed")
        ]
    );
}

/// --hostname-bin
#[derive(Debug)]
pub(in crate::search::rg::flags) struct HostnameBin;

impl Flag for HostnameBin {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "hostname-bin"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("COMMAND")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Run a program to get this system's hostname."
    }
    fn doc_long(&self) -> &'static str {
        r#"
This flag controls how ripgrep determines this system's hostname. The flag's
value should correspond to an executable (either a path or something that can
be found via your system's \fBPATH\fP environment variable). When set, ripgrep
will run this executable, with no arguments, and treat its output (with leading
and trailing whitespace stripped) as your system's hostname.
.sp
When not set (the default, or the empty string), ripgrep will try to
automatically detect your system's hostname. On Unix, this corresponds
to calling \fBgethostname\fP. On Windows, this corresponds to calling
\fBGetComputerNameExW\fP to fetch the system's "physical DNS hostname."
.sp
ripgrep uses your system's hostname for producing hyperlinks.
"#
    }
    fn completion_type(&self) -> CompletionType {
        CompletionType::Executable
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let path = PathBuf::from(v.unwrap_value());
        args.hostname_bin =
            if path.as_os_str().is_empty() { None } else { Some(path) };
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_hostname_bin() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.hostname_bin);

    let args = parse_low_raw(["--hostname-bin", "foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(PathBuf::from("foo")), args.hostname_bin);

    let args = parse_low_raw(["--hostname-bin=foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(PathBuf::from("foo")), args.hostname_bin);
}

/// --hyperlink-format
#[derive(Debug)]
pub(in crate::search::rg::flags) struct HyperlinkFormat;

impl Flag for HyperlinkFormat {
    fn is_switch(&self) -> bool {
        false
    }
    fn name_long(&self) -> &'static str {
        "hyperlink-format"
    }
    fn doc_variable(&self) -> Option<&'static str> {
        Some("FORMAT")
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Set the format of hyperlinks."
    }
    fn doc_long(&self) -> &'static str {
        static DOC: LazyLock<String> = LazyLock::new(|| {
            let mut doc = String::new();
            doc.push_str(
                r#"
Set the format of hyperlinks to use when printing results. Hyperlinks make
certain elements of ripgrep's output, such as file paths, clickable. This
generally only works in terminal emulators that support OSC-8 hyperlinks. For
example, the format \fBfile://{host}{path}\fP will emit an RFC 8089 hyperlink.
To see the format that ripgrep is using, pass the \flag{debug} flag.
.sp
Alternatively, a format string may correspond to one of the following aliases:
"#,
            );

            let mut aliases = grep::printer::hyperlink_aliases();
            aliases.sort_by_key(|alias| {
                alias.display_priority().unwrap_or(i16::MAX)
            });
            for (i, alias) in aliases.iter().enumerate() {
                doc.push_str(r"\fB");
                doc.push_str(alias.name());
                doc.push_str(r"\fP");
                doc.push_str(if i < aliases.len() - 1 { ", " } else { "." });
            }
            doc.push_str(
                r#"
The alias will be replaced with a format string that is intended to work for
the corresponding application.
.sp
The following variables are available in the format string:
.sp
.TP 12
\fB{path}\fP
Required. This is replaced with a path to a matching file. The path is
guaranteed to be absolute and percent encoded such that it is valid to put into
a URI. Note that a path is guaranteed to start with a /.
.TP 12
\fB{host}\fP
Optional. This is replaced with your system's hostname. On Unix, this
corresponds to calling \fBgethostname\fP. On Windows, this corresponds to
calling \fBGetComputerNameExW\fP to fetch the system's "physical DNS hostname."
Alternatively, if \flag{hostname-bin} was provided, then the hostname returned
from the output of that program will be returned. If no hostname could be
found, then this variable is replaced with the empty string.
.TP 12
\fB{line}\fP
Optional. If appropriate, this is replaced with the line number of a match. If
no line number is available (for example, if \fB\-\-no\-line\-number\fP was
given), then it is automatically replaced with the value 1.
.TP 12
\fB{column}\fP
Optional, but requires the presence of \fB{line}\fP. If appropriate, this is
replaced with the column number of a match. If no column number is available
(for example, if \fB\-\-no\-column\fP was given), then it is automatically
replaced with the value 1.
.TP 12
\fB{wslprefix}\fP
Optional. This is a special value that is set to
\fBwsl$/\fP\fIWSL_DISTRO_NAME\fP, where \fIWSL_DISTRO_NAME\fP corresponds to
the value of the equivalent environment variable. If the system is not Unix
or if the \fIWSL_DISTRO_NAME\fP environment variable is not set, then this is
replaced with the empty string.
.PP
A format string may be empty. An empty format string is equivalent to the
\fBnone\fP alias. In this case, hyperlinks will be disabled.
.sp
At present, ripgrep does not enable hyperlinks by default. Users must opt into
them. If you aren't sure what format to use, try \fBdefault\fP.
.sp
Like colors, when ripgrep detects that stdout is not connected to a tty, then
hyperlinks are automatically disabled, regardless of the value of this flag.
Users can pass \fB\-\-color=always\fP to forcefully emit hyperlinks.
.sp
Note that hyperlinks are only written when a path is also in the output
and colors are enabled. To write hyperlinks without colors, you'll need to
configure ripgrep to not colorize anything without actually disabling all ANSI
escape codes completely:
.sp
.EX
    \-\-colors 'path:none' \\
    \-\-colors 'line:none' \\
    \-\-colors 'column:none' \\
    \-\-colors 'match:none'
.EE
.sp
ripgrep works this way because it treats the \flag{color} flag as a proxy for
whether ANSI escape codes should be used at all. This means that environment
variables like \fBNO_COLOR=1\fP and \fBTERM=dumb\fP not only disable colors,
but hyperlinks as well. Similarly, colors and hyperlinks are disabled when
ripgrep is not writing to a tty. (Unless one forces the issue by setting
\fB\-\-color=always\fP.)
.sp
If you're searching a file directly, for example:
.sp
.EX
    rg foo path/to/file
.EE
.sp
then hyperlinks will not be emitted since the path given does not appear
in the output. To make the path appear, and thus also a hyperlink, use the
\flag{with-filename} flag.
.sp
For more information on hyperlinks in terminal emulators, see:
https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda
"#,
            );
            doc
        });
        &DOC
    }

    fn doc_choices(&self) -> &'static [&'static str] {
        static CHOICES: LazyLock<Vec<String>> = LazyLock::new(|| {
            let mut aliases = grep::printer::hyperlink_aliases();
            aliases.sort_by_key(|alias| {
                alias.display_priority().unwrap_or(i16::MAX)
            });
            aliases.iter().map(|alias| alias.name().to_string()).collect()
        });
        static BORROWED: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| CHOICES.iter().map(|name| &**name).collect());
        &*BORROWED
    }

    fn update(&self, v: FlagValue, args: &mut LowArgs) -> anyhow::Result<()> {
        let v = v.unwrap_value();
        let string = convert::str(&v)?;
        let format = string.parse().context("invalid hyperlink format")?;
        args.hyperlink_format = format;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_hyperlink_format() {
    let parseformat = |format: &str| {
        format.parse::<grep::printer::HyperlinkFormat>().expect("Test parsing should succeed")
    };

    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(parseformat("none"), args.hyperlink_format);

    let args = parse_low_raw(["--hyperlink-format", "default"]).expect("Test parsing should succeed");
    #[cfg(windows)]
    assert_eq!(parseformat("file://{path}"), args.hyperlink_format);
    #[cfg(not(windows))]
    assert_eq!(parseformat("file://{host}{path}"), args.hyperlink_format);

    let args = parse_low_raw(["--hyperlink-format", "file"]).expect("Test parsing should succeed");
    assert_eq!(parseformat("file://{host}{path}"), args.hyperlink_format);

    let args = parse_low_raw([
        "--hyperlink-format",
        "file",
        "--hyperlink-format=grep+",
    ])
    .expect("Test parsing should succeed");
    assert_eq!(parseformat("grep+://{path}:{line}"), args.hyperlink_format);

    let args =
        parse_low_raw(["--hyperlink-format", "file://{host}{path}#{line}"])
            .expect("Test parsing should succeed");
    assert_eq!(
        parseformat("file://{host}{path}#{line}"),
        args.hyperlink_format
    );

    let result = parse_low_raw(["--hyperlink-format", "file://heythere"]);
    assert!(result.is_err(), "{result:?}");
}
