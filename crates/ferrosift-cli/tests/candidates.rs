//! Candidate-recipe batch through the real CLI.

mod support;

#[test]
fn candidates_prints_observation_table_for_explicit_hypotheses() {
    let directory = support::TempDir::new("candidates");
    let input = directory.write("input.bin", b"Hi");
    let candidates = directory.write(
        "candidates.json",
        r#"{
          "candidates": [
            {
              "id": "to-hex",
              "format": "cyberchef-v11.3",
              "recipe": [{"op":"To Hex","args":["Space",0]}],
              "checks": [
                {"kind":"utf8_text"},
                {"kind":"prefix_hex","hex":"3438"}
              ]
            },
            {
              "id": "unknown-format",
              "format": "nope",
              "recipe": [],
              "checks": []
            }
          ]
        }"#,
    );

    let run = support::run(
        &[
            "candidates",
            "--input-kind",
            "bytes",
            "--input",
            support::path_text(&input),
            "--candidates",
            support::path_text(&candidates),
        ],
        b"",
    );
    assert!(run.status.success(), "{}", support::stderr(&run));
    let stdout = support::stdout(&run);
    assert!(stdout.contains("ferrosift.candidates.v1"), "{stdout}");
    assert!(stdout.contains("\"id\": \"to-hex\""), "{stdout}");
    assert!(
        stdout.contains("not calibrated probabilities"),
        "{stdout}"
    );
    assert!(
        stdout.contains("host.candidates.format_unknown"),
        "{stdout}"
    );
}
