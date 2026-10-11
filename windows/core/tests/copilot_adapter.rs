use ai_usage_core::{
    child_env,
    runner::{capture, Cancellation},
    store,
};
use std::{collections::BTreeMap, path::PathBuf};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "ai-usage-adapter-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/gh.cmd"),
            dir.join("gh.cmd"),
        )
        .unwrap();
        Self(dir)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let (Ok(target), Ok(root)) = (self.0.canonicalize(), std::env::temp_dir().canonicalize())
        {
            if target.parent() == Some(root.as_path())
                && target
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("ai-usage-adapter-")
            {
                let _ = std::fs::remove_dir_all(target);
            }
        }
    }
}

#[test]
fn copilot_child_owns_auth_and_emits_only_usage_json() {
    let fixture = Fixture::new();
    let (configs, _) = store::load(None);
    let mut source = configs
        .iter()
        .find(|c| c.id == "copilot")
        .unwrap()
        .source
        .account("fixture-account");
    let env = child_env::environment(&BTreeMap::new());
    source.env.insert(
        "PATH".into(),
        format!("{};{}", fixture.0.display(), env.get("PATH").unwrap()),
    );
    let answer = capture(&source, &Cancellation::default()).unwrap();
    assert_eq!(answer["copilot_plan"], "pro");
    assert!(!answer.to_string().contains("fixture-only-auth"));
    source
        .env
        .insert("AI_USAGE_TEST_AUTH_FAIL".into(), "1".into());
    assert!(capture(&source, &Cancellation::default()).is_err());
}
