//! Conformance for MAC, hash, and file-path extractors.

use ferrosift_model::{ArgumentValue, Arguments};

mod support;

#[test]
fn extract_mac_addresses_matches_reference() {
    let sample =
        "Hosts aa:bb:cc:dd:ee:ff and AA-BB-CC-DD-EE-FF and aa:bb:cc:dd:ee:ff and 11:22:33:44:55:66";
    assert_eq!(
        support::output_text(support::run(
            "extract.mac@1",
            Arguments::from([
                ("display_total".into(), ArgumentValue::Boolean(false)),
                ("sort".into(), ArgumentValue::Boolean(false)),
                ("unique".into(), ArgumentValue::Boolean(false)),
            ]),
            support::text(sample),
        )),
        "aa:bb:cc:dd:ee:ff\nAA-BB-CC-DD-EE-FF\naa:bb:cc:dd:ee:ff\n11:22:33:44:55:66"
    );
    assert_eq!(
        support::output_text(support::run(
            "extract.mac@1",
            Arguments::from([
                ("display_total".into(), ArgumentValue::Boolean(true)),
                ("sort".into(), ArgumentValue::Boolean(true)),
                ("unique".into(), ArgumentValue::Boolean(true)),
            ]),
            support::text(sample),
        )),
        "Total found: 3\n\n11:22:33:44:55:66\nAA-BB-CC-DD-EE-FF\naa:bb:cc:dd:ee:ff"
    );
}

#[test]
fn extract_hashes_matches_reference_defaults() {
    let sample = "md5 deadbeefcafebabe0123456789abcdef and sha1 0123456789abcdef0123456789abcdef01234567 and again 0123456789abcdef0123456789abcdef01234567";
    assert_eq!(
        support::output_text(support::run(
            "extract.hashes@1",
            Arguments::from([
                ("hash_character_length".into(), ArgumentValue::Integer(40)),
                ("all_hashes".into(), ArgumentValue::Boolean(false)),
                ("display_total".into(), ArgumentValue::Boolean(false)),
            ]),
            support::text(sample),
        )),
        "0123456789abcdef0123456789abcdef01234567\n0123456789abcdef0123456789abcdef01234567"
    );
    assert_eq!(
        support::output_text(support::run(
            "extract.hashes@1",
            Arguments::from([
                ("hash_character_length".into(), ArgumentValue::Integer(32)),
                ("all_hashes".into(), ArgumentValue::Boolean(false)),
                ("display_total".into(), ArgumentValue::Boolean(true)),
            ]),
            support::text(sample),
        )),
        "Total Results: 1\n\ndeadbeefcafebabe0123456789abcdef"
    );
}

#[test]
fn extract_file_paths_matches_reference() {
    let sample = r"See C:\Windows\System32\cmd.exe and /usr/bin/python3.11 and C:\Temp\file.txt";
    assert_eq!(
        support::output_text(support::run(
            "extract.file_paths@1",
            Arguments::from([
                ("windows".into(), ArgumentValue::Boolean(true)),
                ("unix".into(), ArgumentValue::Boolean(true)),
                ("display_total".into(), ArgumentValue::Boolean(false)),
                ("sort".into(), ArgumentValue::Boolean(false)),
                ("unique".into(), ArgumentValue::Boolean(false)),
            ]),
            support::text(sample),
        )),
        "C:\\Windows\\System32\\cmd.exe\n/usr/bin/python3.11\nC:\\Temp\\file.txt"
    );
    assert_eq!(
        support::output_text(support::run(
            "extract.file_paths@1",
            Arguments::from([
                ("windows".into(), ArgumentValue::Boolean(false)),
                ("unix".into(), ArgumentValue::Boolean(true)),
                ("display_total".into(), ArgumentValue::Boolean(false)),
                ("sort".into(), ArgumentValue::Boolean(false)),
                ("unique".into(), ArgumentValue::Boolean(false)),
            ]),
            support::text(sample),
        )),
        "/usr/bin/python3.11"
    );
}
