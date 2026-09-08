//! Preview quoting, cancellation, bounds, and replacement regressions.

use super::command::read_limited_to;
use super::{
    CommandOutput, PreviewContent, append_truncation_notice, expand_preview_command,
    run_preview_command, should_report_command_failure, signature_query, truncated_image_message,
};
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn pending_replacement_keeps_image_until_current_result_arrives() {
    let mut preview = super::PreviewRuntime::new(
        Some("printf replacement".to_string()),
        crate::ui::GraphicsAdapter::None,
        true,
    );
    preview.content = PreviewContent::Image("previous".to_string());
    let ui = crate::ui::DmenuUI::new(
        vec![crate::common::Item::new_simple(
            "next".into(),
            "next".into(),
            1,
        )],
        false,
        false,
    );
    preview.request_if_changed(&ui);
    assert!(matches!(&preview.content, PreviewContent::Image(key) if key == "previous"));
    let result = preview.next_result().await.expect("command should finish");
    preview.apply_result(result);
    assert!(matches!(&preview.content, PreviewContent::Text(text) if text == "replacement"));
    preview.clear_request();
    assert!(matches!(preview.content, PreviewContent::Empty));
}

#[test]
fn command_expansion_uses_environment_variables() {
    let command = expand_preview_command("printf '%s %s %s' {} {q} {n}")
        .expect("unquoted placeholders should expand");

    assert_eq!(
        command,
        "printf '%s %s %s' \"$FSEL_PREVIEW_ITEM\" \"$FSEL_PREVIEW_QUERY\" \"$FSEL_PREVIEW_ORDINAL\""
    );
}

#[test]
fn password_mode_omits_the_query_from_preview_signatures() {
    assert_eq!(signature_query(false, "secret"), "");
    assert_eq!(signature_query(true, "visible"), "visible");
}

#[test]
fn nested_double_quotes_preserve_shell_context() {
    let command = expand_preview_command("echo \"$(printf \"{}\")\"")
        .expect("nested double-quoted placeholders should expand safely");

    assert_eq!(command, "echo \"$(printf \"$FSEL_PREVIEW_ITEM\"\"\")\"");
}

#[test]
fn nested_single_quoted_placeholders_are_rejected() {
    let result = expand_preview_command("echo \"$(printf '{}')\"");

    assert!(result.is_err());
}

#[test]
fn single_quoted_placeholders_are_rejected() {
    let result = expand_preview_command("printf %s '{}'");

    assert!(result.is_err());
}

#[test]
fn heredoc_preview_commands_are_rejected() {
    let result = expand_preview_command("sh <<EOF\necho {}\nEOF");

    assert!(result.is_err());
}

#[test]
fn quoted_shift_operators_are_not_treated_as_heredocs() {
    let command = expand_preview_command("python -c 'print(1 << 8)' {}")
        .expect("quoted shift operators are ordinary command data");

    assert!(command.contains("print(1 << 8)"));
}

#[test]
fn arithmetic_shift_operators_are_not_treated_as_heredocs() {
    let command = expand_preview_command("printf '%s' $((1 << 8)) {}")
        .expect("arithmetic shifts are not heredocs");

    assert!(command.contains("$((1 << 8))"));
}

#[tokio::test]
async fn arithmetic_ordinal_placeholder_uses_an_unquoted_variable() {
    let command = expand_preview_command("printf '%s' $(({n}+1))")
        .expect("ordinal placeholders should expand in arithmetic contexts");

    assert_eq!(command, "printf '%s' $((FSEL_PREVIEW_ORDINAL+1))");
    let output = tokio::process::Command::new("/bin/sh")
        .args(["-c", &command])
        .env("FSEL_PREVIEW_ORDINAL", "4")
        .output()
        .await
        .expect("POSIX preview command should run");

    assert!(output.status.success());
    assert_eq!(output.stdout, b"5");
}

#[test]
fn case_patterns_do_not_close_command_substitutions() {
    let command = expand_preview_command("echo \"$(case x in x) printf '%s' \"{}\";; esac)\"")
        .expect("case pattern terminators belong to the case clause");

    assert!(command.contains("$FSEL_PREVIEW_ITEM"));
}

#[test]
fn case_pattern_parentheses_remain_balanced() {
    let command = expand_preview_command("echo \"$(case x in @(x)) :;; esac){}\"")
        .expect("parentheses within a case pattern should remain balanced");

    assert!(command.ends_with("$FSEL_PREVIEW_ITEM\"\"\""));
}

#[tokio::test]
async fn case_as_an_argument_does_not_change_shell_context() {
    let command = expand_preview_command("printf '<%s>' \"$(printf case){}\"")
        .expect("an ordinary case argument is not case grammar");
    let payload = "two words*.txt";

    let output = run_preview_command(&command, payload, Some(""), 0)
        .await
        .expect("preview command should run");

    assert!(output.success);
    assert_eq!(output.stdout, b"<casetwo words*.txt>");
}

#[tokio::test]
async fn grouped_case_preserves_placeholder_quoting() {
    let command =
        expand_preview_command("printf '<%s>' \"$( (case x in x) :;; esac); printf '%s' {})\"")
            .expect("a grouped case command should keep substitution context");
    let payload = "two words*.txt";

    let output = tokio::process::Command::new("/bin/sh")
        .args(["-c", &command])
        .env("FSEL_PREVIEW_ITEM", payload)
        .env("FSEL_PREVIEW_QUERY", "")
        .env("FSEL_PREVIEW_ORDINAL", "0")
        .output()
        .await
        .expect("POSIX preview command should run");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"<two words*.txt>");
}

#[tokio::test]
async fn grouping_inside_case_preserves_outer_quote_context() {
    let command = expand_preview_command("printf '<%s>' \"$(case x in x) ( : );; esac){}\"")
        .expect("a group inside a case body should not close the substitution");
    let payload = "two words*.txt";

    let output = tokio::process::Command::new("/bin/sh")
        .args(["-c", &command])
        .env("FSEL_PREVIEW_ITEM", payload)
        .output()
        .await
        .expect("POSIX preview command should run");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"<two words*.txt>");
}

#[tokio::test]
async fn leading_case_parenthesis_preserves_outer_quote_context() {
    let command = expand_preview_command("printf '<%s>' \"$(case x in (x) printf x;; esac){}\"")
        .expect("a leading case parenthesis should not close the substitution");
    let payload = "two words*.txt";

    let output = tokio::process::Command::new("/bin/sh")
        .args(["-c", &command])
        .env("FSEL_PREVIEW_ITEM", payload)
        .output()
        .await
        .expect("POSIX preview command should run");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"<xtwo words*.txt>");
}

#[tokio::test]
async fn backtick_substitution_preserves_placeholder_quoting() {
    let command = expand_preview_command("printf '<%s>' \"`printf %s {}`\"")
        .expect("a backtick substitution should have its own quote context");
    let payload = "two words*.txt";

    let output = tokio::process::Command::new("/bin/sh")
        .args(["-c", &command])
        .env("FSEL_PREVIEW_ITEM", payload)
        .output()
        .await
        .expect("POSIX preview command should run");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"<two words*.txt>");
}

#[tokio::test]
async fn selected_item_is_not_reparsed_as_shell_source() {
    let command =
        expand_preview_command("printf '%s' {}").expect("unquoted placeholder should expand");
    let payload = "$(printf injected >&2)";

    let output = run_preview_command(&command, payload, Some(""), 0)
        .await
        .expect("preview command should run");

    assert!(output.success);
    assert_eq!(output.stdout, payload.as_bytes());
    assert!(output.stderr.is_empty());
}

#[tokio::test]
async fn password_mode_does_not_export_the_query() {
    let output = run_preview_command("env", "selected", None, 0)
        .await
        .expect("preview command should run without a query environment variable");

    assert!(output.success);
    assert!(
        !output
            .stdout
            .split(|byte| *byte == b'\n')
            .any(|line| line.starts_with(b"FSEL_PREVIEW_QUERY="))
    );
}

#[tokio::test]
async fn double_quoted_placeholder_preserves_one_argument() {
    let command = expand_preview_command("printf '<%s>' \"{}\"")
        .expect("double-quoted placeholder should expand");
    let payload = "two words*.txt";

    let output = run_preview_command(&command, payload, Some(""), 0)
        .await
        .expect("preview command should run");

    assert!(output.success);
    assert_eq!(output.stdout, b"<two words*.txt>");
}

#[tokio::test]
async fn nested_double_quoted_placeholder_expands_the_row() {
    let command = expand_preview_command("printf '<%s>' \"$(printf '%s' \"{}\")\"")
        .expect("nested double-quoted placeholder should expand");
    let payload = "two words*.txt";

    let output = run_preview_command(&command, payload, Some(""), 0)
        .await
        .expect("preview command should run");

    assert!(output.success);
    assert_eq!(output.stdout, b"<two words*.txt>");
}

#[tokio::test]
async fn limited_reader_drains_after_reporting_the_cap() {
    let (mut writer, reader) = tokio::io::duplex(32);
    let writer_task = tokio::spawn(async move {
        writer
            .write_all(b"abcdef")
            .await
            .expect("write should work");
        writer.shutdown().await.expect("shutdown should work");
    });
    let (limit_tx, mut limit_rx) = tokio::sync::mpsc::channel(2);

    let (bytes, truncated) = read_limited_to(reader, 3, limit_tx)
        .await
        .expect("read should work");
    writer_task.await.expect("writer should finish");

    assert_eq!(bytes, b"abc");
    assert!(truncated);
    assert!(limit_rx.try_recv().is_ok());
}

#[test]
fn truncation_notice_is_added_to_failed_command_diagnostics() {
    let mut text = "command failed".to_string();

    append_truncation_notice(&mut text, true);

    assert_eq!(text, "command failed\n\n[preview output truncated]");
}

#[test]
fn truncated_images_are_reported_before_decode() {
    let output = CommandOutput {
        stdout: b"\x89PNG\r\n\x1a\npartial".to_vec(),
        stderr: Vec::new(),
        status: "signal: 9".to_string(),
        success: false,
        stdout_truncated: true,
        stderr_truncated: false,
    };

    assert_eq!(
        truncated_image_message(&output).as_deref(),
        Some("Preview image exceeds the 32 MiB output limit")
    );
}

#[test]
fn truncated_stderr_does_not_reject_a_complete_image() {
    let output = CommandOutput {
        stdout: b"\x89PNG\r\n\x1a\ncomplete".to_vec(),
        stderr: b"diagnostic".to_vec(),
        status: "signal: 9".to_string(),
        success: false,
        stdout_truncated: false,
        stderr_truncated: true,
    };

    assert_eq!(truncated_image_message(&output), None);
}

#[test]
fn nonzero_commands_keep_nonempty_stdout() {
    let output = CommandOutput {
        stdout: b"useful diff".to_vec(),
        stderr: Vec::new(),
        status: "exit status: 1".to_string(),
        success: false,
        stdout_truncated: false,
        stderr_truncated: false,
    };

    assert!(!should_report_command_failure(&output));
}
