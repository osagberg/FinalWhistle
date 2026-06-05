//! BK-E-3 regression: unimplemented subcommands must return Err (non-zero exit),
//! not Ok (silent success).
//!
//! Tests exercise `fw_content_baker::stub_unimplemented` directly — the same
//! function the binary's `main()` calls for every deferred subcommand — so the
//! Err-propagation path from dispatch to process exit is covered by the real
//! code, not a re-implementation.

use fw_content_baker::stub_unimplemented;

// ---------------------------------------------------------------------------
// BK-E-3 core: stub returns Err with an informative message
// ---------------------------------------------------------------------------

#[test]
fn stub_returns_err_not_ok() {
    let result = stub_unimplemented("bake-bios", "T4.5-D");
    assert!(
        result.is_err(),
        "stub_unimplemented must return Err so the process exits non-zero"
    );
}

#[test]
fn stub_error_message_contains_subcommand_name() {
    let err = stub_unimplemented("bake-headlines", "T4.5-D").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("bake-headlines"),
        "error message must name the subcommand; got: {msg:?}"
    );
}

#[test]
fn stub_error_message_contains_milestone() {
    let err = stub_unimplemented("bake-all", "T4.5-D (full pipeline orchestration)").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("T4.5-D"),
        "error message must name the milestone; got: {msg:?}"
    );
}

// ---------------------------------------------------------------------------
// Verify that all 8 deferred subcommand names produce Err (exhaustive list
// matches the match arms in main.rs so a new stub that forgets to call
// stub_unimplemented would NOT be caught here — but any existing stub that
// accidentally reverts to Ok would be caught immediately).
// ---------------------------------------------------------------------------

#[test]
fn all_deferred_subcommands_return_err() {
    let stubs: &[(&str, &str)] = &[
        ("bake-bios", "T4.5-D"),
        ("bake-headlines", "T4.5-D"),
        ("bake-scout-phrases", "T4.5-D"),
        ("bake-manager-quotes", "T4.5-D"),
        ("bake-fan-reactions", "T4.5-D"),
        ("bake-commentary", "T4.5-D"),
        ("bake-all", "T4.5-D (full pipeline orchestration)"),
        ("manifest", "T4.5-D"),
    ];

    for (cmd, milestone) in stubs {
        assert!(
            stub_unimplemented(cmd, milestone).is_err(),
            "stub_unimplemented(\"{cmd}\", \"{milestone}\") must return Err"
        );
    }
}
