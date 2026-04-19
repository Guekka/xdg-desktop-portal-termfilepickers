use std::process::Command;

fn has_nushell() -> bool {
    Command::new("nu")
        .arg("--version")
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn yazi_open_file_wrapper_accepts_json_arguments() {
    if !has_nushell() {
        return;
    }

    let script = format!(
        "{}/data/share/wrappers/yazi-open-file.nu",
        env!("CARGO_MANIFEST_DIR")
    );
    let temp = std::env::temp_dir();
    let payload = serde_json::json!({
        "out_file": temp.join("out-file"),
        "termcmd": ["true"],
        "directory": false
    })
    .to_string();

    let status = Command::new("nu")
        .arg(script)
        .arg(payload)
        .status()
        .expect("failed to execute yazi-open-file wrapper");

    assert!(status.success(), "wrapper failed with status: {status}");
}

#[test]
fn yazi_save_file_wrapper_accepts_json_arguments() {
    if !has_nushell() {
        return;
    }

    let script = format!(
        "{}/data/share/wrappers/yazi-save-file.nu",
        env!("CARGO_MANIFEST_DIR")
    );
    let temp = std::env::temp_dir();
    let payload = serde_json::json!({
        "out_file": temp.join("out-file"),
        "termcmd": ["true"],
        "recommended_path": temp.join("target-file")
    })
    .to_string();

    let status = Command::new("nu")
        .arg(script)
        .arg(payload)
        .status()
        .expect("failed to execute yazi-save-file wrapper");

    assert!(status.success(), "wrapper failed with status: {status}");
}
