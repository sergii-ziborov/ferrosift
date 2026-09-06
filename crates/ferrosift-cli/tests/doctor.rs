//! Install smoke checks through the real CLI process.

mod support;

#[test]
fn doctor_reports_ok_install() {
    let output = support::run(&["doctor"], b"");
    assert!(output.status.success(), "{}", support::stderr(&output));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_eq!(report["schema"], "ferrosift.doctor.v1");
    assert_eq!(report["ok"], true);
    assert_eq!(report["version"], env!("CARGO_PKG_VERSION"));
    let checks = report["checks"].as_array().expect("checks");
    let ids: Vec<&str> = checks
        .iter()
        .map(|check| check["id"].as_str().expect("id"))
        .collect();
    assert!(ids.contains(&"cli.version"));
    assert!(ids.contains(&"registry.load"));
    assert!(ids.contains(&"recipe.smoke"));
    assert!(ids.contains(&"pattern.smoke"));
    assert!(checks.iter().all(|check| check["ok"] == true));
}
