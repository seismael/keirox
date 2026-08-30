//! Automated doc link validator ensuring zero broken internal markdown links.

use std::fs;
use std::path::{Path, PathBuf};

fn verify_markdown_file_links(file_path: &Path, root_dir: &Path) {
    let content = fs::read_to_string(file_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", file_path.display()));
    let parent = file_path.parent().unwrap();

    // Look for markdown links: [text](link)
    let mut rest = content.as_str();
    while let Some(start_bracket) = rest.find('[') {
        rest = &rest[start_bracket + 1..];
        if let Some(end_bracket) = rest.find(']') {
            let after_bracket = &rest[end_bracket + 1..];
            if after_bracket.starts_with('(') {
                if let Some(end_paren) = after_bracket.find(')') {
                    let raw_link = &after_bracket[1..end_paren];
                    // Clean anchor #...
                    let link_target = raw_link.split('#').next().unwrap_or("").trim();

                    // Only check local relative file links (ignore http/https/mailto)
                    if !link_target.is_empty()
                        && !link_target.starts_with("http")
                        && !link_target.starts_with("mailto:")
                    {
                        let target_path = if let Some(stripped) = link_target.strip_prefix('/') {
                            root_dir.join(stripped)
                        } else {
                            parent.join(link_target)
                        };

                        assert!(
                            target_path.exists(),
                            "Broken link in {}: '{}' resolves to non-existent path '{}'",
                            file_path.display(),
                            raw_link,
                            target_path.display()
                        );
                    }
                    rest = &after_bracket[end_paren + 1..];
                    continue;
                }
            }
            rest = &rest[end_bracket + 1..];
        }
    }
}

#[test]
fn test_all_key_markdown_links_are_valid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let files_to_check = [
        root.join("README.md"),
        root.join("AGENTS.md"),
        root.join("CONTRIBUTING.md"),
        root.join("docs/architecture/INDEX.md"),
        root.join("docs/engineering/README.md"),
        root.join("docs/benchmarks/README.md"),
        root.join("docs/reports/README.md"),
        root.join("docs/archive/README.md"),
        root.join("scripts/README.md"),
        root.join("deploy/README.md"),
        root.join("tests/integration/README.md"),
        root.join("tests/golden/README.md"),
        root.join("tests/chaos/README.md"),
        root.join("tests/soak/README.md"),
    ];

    for file in &files_to_check {
        if file.exists() {
            verify_markdown_file_links(file, &root);
        }
    }
}
