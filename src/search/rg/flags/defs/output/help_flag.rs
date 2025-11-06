//! Help flag.

use crate::search::rg::flags::{
    Category, Flag, FlagValue,
    lowargs::LowArgs,
};

/// -h/--help
#[derive(Debug)]
pub(in crate::search::rg::flags) struct Help;

impl Flag for Help {
    fn is_switch(&self) -> bool {
        true
    }
    fn name_long(&self) -> &'static str {
        "help"
    }
    fn name_short(&self) -> Option<u8> {
        Some(b'h')
    }
    fn doc_category(&self) -> Category {
        Category::Output
    }
    fn doc_short(&self) -> &'static str {
        r"Show help output."
    }
    fn doc_long(&self) -> &'static str {
        r"
This flag prints the help output for ripgrep.
.sp
Unlike most other flags, the behavior of the short flag, \fB\-h\fP, and the
long flag, \fB\-\-help\fP, is different. The short flag will show a condensed
help output while the long flag will show a verbose help output. The verbose
help output has complete documentation, where as the condensed help output will
show only a single line for every flag.
"
    }

    fn update(&self, v: FlagValue, _: &mut LowArgs) -> anyhow::Result<()> {
        assert!(v.unwrap_switch(), "--help has no negation");
        // Since this flag has different semantics for -h and --help and the
        // Flag trait doesn't support encoding this sort of thing, we handle it
        // as a special case in the parser.
        Ok(())
    }
}
