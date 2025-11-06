/*!
Tests for HiArgs binary mode detection.
*/

#[cfg(test)]
mod binary_mode_tests {
    use crate::search::rg::flags::{
        hiargs::{types::BinaryDetection, types::State},
        lowargs::{BinaryMode as RgBinaryMode, LowArgs},
    };

    /// Test that `BinaryMode::Auto` maps to `RgBinaryMode::Auto`
    /// This is the default: skip binary files
    #[test]
    fn test_binary_mode_auto_maps_to_rg_auto() {
        let low = LowArgs {
            binary: RgBinaryMode::Auto,
            ..Default::default()
        };

        let state = State::new().expect("Failed to create state");
        let detection = BinaryDetection::from_low_args(&state, &low);

        // For Auto mode:
        // - explicit files use convert (search with NUL replacement)
        // - implicit files use quit (skip binaries)
        let expected_explicit = grep::searcher::BinaryDetection::convert(b'\x00');
        let expected_implicit = grep::searcher::BinaryDetection::quit(b'\x00');

        assert_eq!(
            detection.explicit, expected_explicit,
            "Auto mode: explicit detection should use convert"
        );
        assert_eq!(
            detection.implicit, expected_implicit,
            "Auto mode: implicit detection should use quit"
        );
    }

    /// Test that `BinaryMode::Binary` maps to `RgBinaryMode::SearchAndSuppress`
    /// Matches rg --binary behavior
    #[test]
    fn test_binary_mode_binary_maps_to_search_and_suppress() {
        let low = LowArgs {
            binary: RgBinaryMode::SearchAndSuppress,
            ..Default::default()
        };

        let state = State::new().expect("Failed to create state");
        let detection = BinaryDetection::from_low_args(&state, &low);

        // For SearchAndSuppress mode:
        // - both explicit and implicit use convert (search with NUL replacement)
        let expected = grep::searcher::BinaryDetection::convert(b'\x00');

        assert_eq!(
            detection.explicit, expected,
            "Binary mode: explicit detection should use convert"
        );
        assert_eq!(
            detection.implicit, expected,
            "Binary mode: implicit detection should use convert"
        );
    }

    /// Test that `BinaryMode::Text` maps to `RgBinaryMode::AsText`
    /// Matches rg -a/--text behavior
    #[test]
    fn test_binary_mode_text_maps_to_as_text() {
        let low = LowArgs {
            binary: RgBinaryMode::AsText,
            ..Default::default()
        };

        let state = State::new().expect("Failed to create state");
        let detection = BinaryDetection::from_low_args(&state, &low);

        // For AsText mode:
        // - both explicit and implicit use none (no binary detection)
        let expected = grep::searcher::BinaryDetection::none();

        assert_eq!(
            detection.explicit, expected,
            "Text mode: explicit detection should be none"
        );
        assert_eq!(
            detection.implicit, expected,
            "Text mode: implicit detection should be none"
        );
        assert!(
            detection.is_none(),
            "Text mode: is_none() should return true"
        );
    }
}
