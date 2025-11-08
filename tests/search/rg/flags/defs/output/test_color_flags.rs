//! Tests for color and hyperlink output flags

use kodegen_tools_filesystem::search::rg::flags::parse::parse_low_raw;
use kodegen_tools_filesystem::search::rg::flags::ColorChoice;
use std::path::PathBuf;

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

#[test]
fn test_hostname_bin() {
    let args = parse_low_raw(None::<&str>).expect("Test parsing should succeed");
    assert_eq!(None, args.hostname_bin);

    let args = parse_low_raw(["--hostname-bin", "foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(PathBuf::from("foo")), args.hostname_bin);

    let args = parse_low_raw(["--hostname-bin=foo"]).expect("Test parsing should succeed");
    assert_eq!(Some(PathBuf::from("foo")), args.hostname_bin);
}

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
