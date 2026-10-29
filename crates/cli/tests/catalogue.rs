use assert_cmd::{Command, cargo::cargo_bin_cmd};
#[test]
fn static_catalogue_export_matches_common_api_and_checked_documentation() {
    let mut c: Command = cargo_bin_cmd!("typstographe-cli");
    c.args(["commands", "export", "--markdown"])
        .assert()
        .success()
        .stdout(include_str!("../../../docs/cli.md").replace("\r\n", "\n"));
    let output = cargo_bin_cmd!("typstographe-cli")
        .args(["--json", "commands", "export"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(v["outcome"], typstographe_core::content::api::catalogue());
    assert_eq!(
        v["outcome"],
        serde_json::from_str::<serde_json::Value>(include_str!("../../../docs/commands.json"))
            .unwrap()
    );
}
