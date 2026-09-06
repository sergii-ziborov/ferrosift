//! Artifact store and allowlist security checks.

use std::{
    fs,
    path::Path,
    time::Duration,
};

use ferrosift_host::{
    ArtifactStore, HostConfig, HostService, InputKind, InspectRequest, OpenRequest, PathAllowlist,
    RecipeFormat, RunRequest, StoreConfig,
};
use ferrosift_model::Value;

#[test]
fn allowlist_rejects_paths_outside_configured_roots() {
    let root = tempfile::tempdir().expect("temp root");
    let outside = tempfile::tempdir().expect("outside");
    let allowed = PathAllowlist::new([root.path()]).expect("roots");
    let forbidden = outside.path().join("secret.bin");
    fs::write(&forbidden, b"nope").expect("write");
    let error = allowed
        .read_limited(&forbidden, 1024)
        .expect_err("outside root must fail");
    assert_eq!(error.code(), "host.path.access_denied");
}

#[test]
fn allowlist_rejects_parent_traversal_that_escapes_the_root() {
    let root = tempfile::tempdir().expect("temp root");
    let sibling = root.path().parent().expect("parent").join("escape-target.bin");
    fs::write(&sibling, b"escaped").expect("write sibling");
    let allowed = PathAllowlist::new([root.path()]).expect("roots");
    let probe = root.path().join("..").join(
        sibling
            .file_name()
            .expect("name"),
    );
    // canonicalize may succeed; access must still fail because the resolved
    // path is not under the allowlisted root.
    if let Ok(resolved) = probe.canonicalize()
        && !resolved.starts_with(root.path().canonicalize().expect("root"))
    {
        let error = allowed
            .read_limited(&probe, 1024)
            .expect_err("escaped path must fail");
        assert_eq!(error.code(), "host.path.access_denied");
    }
}

#[cfg(unix)]
#[test]
fn allowlist_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("temp root");
    let outside = tempfile::tempdir().expect("outside");
    let target = outside.path().join("secret.bin");
    fs::write(&target, b"secret").expect("write");
    let link = root.path().join("link.bin");
    symlink(&target, &link).expect("symlink");
    let allowed = PathAllowlist::new([root.path()]).expect("roots");
    let error = allowed
        .read_limited(&link, 1024)
        .expect_err("symlink escape must fail");
    assert_eq!(error.code(), "host.path.access_denied");
}

#[test]
fn store_expires_handles_and_enforces_quota() {
    let mut store = ArtifactStore::new(StoreConfig {
        max_total_bytes: 8,
        ttl: Duration::from_millis(20),
        max_preview_bytes: 4,
    });
    let meta = store
        .insert(Value::Bytes(vec![1, 2, 3, 4]))
        .expect("insert");
    assert!(store.meta(&meta.id).is_ok());
    std::thread::sleep(Duration::from_millis(40));
    let error = store.meta(&meta.id).expect_err("ttl elapsed");
    assert_eq!(error.code(), "host.artifact.expired");

    let mut tight = ArtifactStore::new(StoreConfig {
        max_total_bytes: 4,
        ttl: Duration::from_mins(1),
        max_preview_bytes: 4,
    });
    tight
        .insert(Value::Bytes(vec![1, 2, 3, 4]))
        .expect("exact quota");
    let error = tight
        .insert(Value::Bytes(vec![1]))
        .expect_err("over quota");
    assert_eq!(error.code(), "host.artifact.quota_exceeded");
}

#[test]
fn host_open_run_inspect_keeps_bytes_behind_handles() {
    let root = tempfile::tempdir().expect("samples");
    let sample = root.path().join("payload.bin");
    fs::write(&sample, b"Hi").expect("sample");

    let service = HostService::new(HostConfig {
        allowed_roots: vec![root.path().to_path_buf()],
        ..HostConfig::default()
    })
    .expect("host");

    let opened = service
        .open(OpenRequest::Path {
            path: &sample,
            kind: InputKind::Bytes,
        })
        .expect("open");
    let inspect = service
        .inspect(
            opened.id.as_str(),
            InspectRequest {
                include_preview: true,
            },
        )
        .expect("inspect");
    assert_eq!(inspect.value_kind, ferrosift_model::ValueKind::Bytes);
    assert_eq!(inspect.preview_hex.as_deref(), Some("4869"));

    let recipe = br#"[{"op":"To Hex","args":["Space",0]}]"#;
    let report = service
        .run(&RunRequest {
            recipe,
            format: RecipeFormat::CyberChefV11_3,
            input_artifact_id: opened.id.as_str(),
        })
        .expect("run");
    assert_eq!(report.schema, "ferrosift.execution.v1");
    assert!(matches!(
        report.status,
        ferrosift_host::ReportStatus::Completed
    ));

    let hits = service.search("hex");
    assert!(!hits.is_empty());
    let described = service
        .describe(&hits[0].id)
        .expect("describe");
    assert!(!described.description.is_empty());
}

#[test]
fn host_open_rejects_unconfigured_paths() {
    let service = HostService::new(HostConfig::default()).expect("host");
    let error = service
        .open(OpenRequest::Path {
            path: Path::new("nope.bin"),
            kind: InputKind::Bytes,
        })
        .expect_err("no roots");
    assert_eq!(error.code(), "host.path.access_denied");
}
