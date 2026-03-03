//! Integration tests running the bridge-registry CLI.

use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_bridge-registry");

fn run(args: &[&str]) -> (bool, String) {
    let output = Command::new(BINARY)
        .args(args)
        .output()
        .expect("failed to run binary");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let success = output.status.success();
    let out = if success {
        stdout
    } else {
        format!("{stdout}\n{stderr}")
    };
    (success, out)
}

#[test]
#[ignore = "requires network"]
fn test_bridges_then_tokens_on_chain_37() {
    const CHAIN_ID: u32 = 37;
    let chain_prefix = format!("eip155:{CHAIN_ID}:");

    let (ok, out) = run(&["bridges", "37"]);
    assert!(ok, "bridges {CHAIN_ID} failed: {out}");

    let bridges: Vec<String> =
        serde_json::from_str(&out).expect("bridges output must be JSON array");
    assert!(
        !bridges.is_empty(),
        "chain {CHAIN_ID} should be supported by at least one bridge"
    );

    let mut any_have_tokens = false;
    for bridge in &bridges {
        let (ok, out) = run(&["tokens", bridge]);
        assert!(ok, "tokens {bridge} failed: {out}");

        let tokens: Vec<TokenInfo> =
            serde_json::from_str(&out).expect("tokens output must be JSON array");
        let on_chain: Vec<_> = tokens
            .iter()
            .filter(|t| t.id.starts_with(&chain_prefix))
            .collect();
        if !on_chain.is_empty() {
            any_have_tokens = true;
        }
    }
    assert!(
        any_have_tokens,
        "at least one bridge supporting chain {CHAIN_ID} should have tokens on it"
    );
}

#[derive(serde::Deserialize)]
struct TokenInfo {
    id: String,
}
