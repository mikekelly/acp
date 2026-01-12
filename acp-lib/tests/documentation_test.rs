/// Tests for documentation files
///
/// These tests verify that critical documentation files exist and contain expected content.
/// While not traditional "behavior" tests, they ensure documentation stays in sync with code.

use std::fs;
use std::path::Path;

#[test]
fn test_gotchas_documentation_exists() {
    let gotchas_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("docs/reference/gotchas.md");

    assert!(
        gotchas_path.exists(),
        "docs/reference/gotchas.md should exist"
    );

    let content = fs::read_to_string(&gotchas_path)
        .expect("Should be able to read gotchas.md");

    // Verify it's a proper markdown document with title
    assert!(
        content.contains("# Gotchas"),
        "Document should have a title"
    );

    // Verify key gotchas are documented (spot check a few critical ones)
    assert!(
        content.contains("Wildcard matching is single-level only"),
        "Should document wildcard matching limitation"
    );

    assert!(
        content.contains("PluginRuntime is not Send"),
        "Should document PluginRuntime Send limitation"
    );

    assert!(
        content.contains("Token field access"),
        "Should document token field access pattern"
    );

    // Verify it has proper structure
    assert!(
        content.contains("##"),
        "Document should have section headings"
    );
}
