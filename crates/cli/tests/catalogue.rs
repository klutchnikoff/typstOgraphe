use assert_cmd::Command;

#[test]
fn static_catalogue_exports_no_longer_require_a_graphical_application() {
    let output = Command::new(assert_cmd::cargo::cargo_bin!("typstographe-cli"))
        .args(["commands", "export", "--markdown"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(
        String::from_utf8(output).unwrap(),
        include_str!("../../../docs/commands.md").replace("\r\n", "\n")
    );
    let output = Command::new(assert_cmd::cargo::cargo_bin!("typstographe-cli"))
        .args(["--json", "commands", "export"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        value["outcome"],
        typstographe_core::engine::registry::catalogue_json()
    );
}
