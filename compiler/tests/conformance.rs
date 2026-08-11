//! The MLang conformance suite.
//!
//! ../conformance/cases.json holds the corpus (inline sources and example
//! files); ../conformance/expected.json holds the recorded observable
//! behavior — stdout, stderr, and exit code, byte for byte. These goldens
//! ARE the language's observable specification.
//!
//! To re-record after an intentional behavior change:
//!     RECORD=1 cargo test --release conformance
//! then review the diff of expected.json before committing.

use serde_json::{json, Map, Value};

fn root(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel)
}

fn load(rel: &str) -> Value {
    let text = std::fs::read_to_string(root(rel)).unwrap();
    serde_json::from_str(&text).unwrap()
}

struct Job {
    name: String,
    source: String,
    stdin: String,
}

fn jobs() -> Vec<Job> {
    let cases = load("conformance/cases.json");
    let mut jobs = Vec::new();
    for c in cases["cases"].as_array().unwrap() {
        jobs.push(Job {
            name: c["name"].as_str().unwrap().to_string(),
            source: c["source"].as_str().unwrap().to_string(),
            stdin: c["stdin"].as_str().unwrap().to_string(),
        });
    }
    for e in cases["examples"].as_array().unwrap() {
        let file = e["file"].as_str().unwrap();
        jobs.push(Job {
            name: format!("example:{}", e["name"].as_str().unwrap()),
            source: std::fs::read_to_string(root(file)).unwrap(),
            stdin: e["stdin"].as_str().unwrap().to_string(),
        });
    }
    jobs
}

#[test]
fn conformance() {
    let jobs = jobs();
    if std::env::var("RECORD").is_ok() {
        let mut expected = Map::new();
        for job in &jobs {
            let (code, out, err) = mlang::run_text(&job.source, &job.stdin);
            expected.insert(
                job.name.clone(),
                json!({"exit": code, "stdout": out, "stderr": err}),
            );
        }
        let mut text = serde_json::to_string_pretty(&Value::Object(expected)).unwrap();
        text.push('\n');
        std::fs::write(root("conformance/expected.json"), text).unwrap();
        println!("recorded {} cases", jobs.len());
        return;
    }

    let expected = load("conformance/expected.json");
    let mut failed = Vec::new();
    for job in &jobs {
        let want = &expected[&job.name];
        assert!(!want.is_null(), "no expectation recorded for {}", job.name);
        let (code, out, err) = mlang::run_text(&job.source, &job.stdin);
        let ok = code as i64 == want["exit"].as_i64().unwrap()
            && out == want["stdout"].as_str().unwrap()
            && err == want["stderr"].as_str().unwrap();
        if !ok {
            failed.push(job.name.clone());
            eprintln!("✗ {}", job.name);
            eprintln!("    want exit {} stdout {:?} stderr {:?}",
                      want["exit"], want["stdout"], want["stderr"]);
            eprintln!("     got exit {code} stdout {out:?} stderr {err:?}");
        }
    }
    assert!(
        failed.is_empty(),
        "{}/{} conformance cases failed: {}",
        failed.len(),
        jobs.len(),
        failed.join(", ")
    );
    println!("{}/{} conformance cases pass", jobs.len(), jobs.len());
}
