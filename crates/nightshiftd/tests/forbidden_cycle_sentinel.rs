//! Canonical forbidden-cycle sentinel.
//!
//! NQ diagnostic artifacts flow into Nightshift as immutable inputs. Nothing
//! in canonical production may publish Nightshift posture back into NQ truth.

use std::fs;
use std::path::{Path, PathBuf};

fn visit(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            visit(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn canonical_nightshift_has_no_nq_truth_write_back_edge() {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    visit(&source_root, &mut files);
    files.sort();

    let forbidden_nq_verbs = [
        "emit-finding",
        "create-finding",
        "push-posture",
        "report-closure",
        "report-state",
        "report-attention",
        "report-silence",
        "ack-finding",
    ];
    for path in files {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        for verb in forbidden_nq_verbs {
            assert!(
                !source.contains(&format!("\"{verb}\"")),
                "{} contains forbidden NQ write verb {verb}",
                path.display()
            );
        }
        assert!(
            !source.contains("Connection::open(nq_") && !source.contains("Connection::open(&nq_"),
            "{} directly opens an NQ database locator",
            path.display()
        );
    }
}
