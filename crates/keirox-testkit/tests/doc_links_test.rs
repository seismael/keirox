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

                    // Only check local relative file links (ignore http/https/mailto and documentation placeholders)
                    if !link_target.is_empty()
                        && !link_target.starts_with("http")
                        && !link_target.starts_with("mailto:")
                        && link_target != "path"
                        && link_target != "url"
                        && link_target != "file"
                        && link_target != "target"
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

fn find_all_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if dir_name != "target" && dir_name != ".git" && dir_name != "node_modules" {
                    find_all_markdown_files(&path, files);
                }
            } else if path.extension().is_some_and(|ext| ext == "md") {
                files.push(path);
            }
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

    let mut all_md_files = Vec::new();
    find_all_markdown_files(&root, &mut all_md_files);

    assert!(
        all_md_files.len() >= 30,
        "Expected at least 30 markdown files in the repo, found {}",
        all_md_files.len()
    );

    for file in &all_md_files {
        verify_markdown_file_links(file, &root);
    }
}
