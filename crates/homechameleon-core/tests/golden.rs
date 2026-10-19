//! Byte-for-byte theme output. Regenerate with UPDATE_GOLDEN=1.

use std::path::PathBuf;

use homechameleon_core::fixture_report;

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/fixture.txt")
}

#[test]
fn fixture_matches_golden() {
    let report = fixture_report();
    let path = golden_path();
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &report).unwrap();
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "missing golden {}: {err}. Run with UPDATE_GOLDEN=1",
            path.display()
        )
    });
    assert_eq!(report, expected);
}

#[test]
fn pyyaml_accepts_every_fixture_theme() {
    let report = fixture_report();
    let mut themes = Vec::new();
    for chunk in report
        .split("===== ")
        .filter(|chunk| chunk.contains("modes:\n"))
    {
        let yaml = chunk.split("--- css ---\n").next().unwrap();
        let yaml = yaml.split_once('\n').map(|(_, rest)| rest).unwrap_or(yaml);
        themes.push(yaml.to_string());
    }
    assert!(
        themes.len() >= 4,
        "expected several themes, got {}",
        themes.len()
    );
    let joined = themes.join("\n---\n");
    let path = std::env::temp_dir().join("homechameleon-themes.yaml");
    std::fs::write(&path, &joined).unwrap();
    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg("import sys, yaml; docs = list(yaml.safe_load_all(open(sys.argv[1]))); assert docs and all(doc and any(isinstance(v, dict) and 'modes' in v for v in doc.values()) for doc in docs); print(len(docs))")
        .arg(&path)
        .output()
        .expect("python3");
    assert!(
        output.status.success(),
        "pyyaml failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
