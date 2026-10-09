use super::*;
use anyhow::Context;
use serde_json::Value;
use std::collections::BTreeSet;

fn text<'a>(fields: &'a Value, name: &str) -> anyhow::Result<&'a str> {
    fields
        .get(name)
        .and_then(Value::as_str)
        .with_context(|| format!("missing string {name}"))
}

fn number(fields: &Value, name: &str) -> anyhow::Result<u32> {
    let value = fields
        .get(name)
        .and_then(Value::as_u64)
        .with_context(|| format!("missing integer {name}"))?;
    Ok(u32::try_from(value)?)
}

fn hex(value: &str) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(value.len().is_multiple_of(2), "odd hex length");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect()
}

fn bytes(fields: &Value, name: &str) -> anyhow::Result<Vec<u8>> {
    hex(text(fields, name)?)
}

fn array<const N: usize>(fields: &Value, name: &str) -> anyhow::Result<[u8; N]> {
    bytes(fields, name)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("wrong array size for {name}"))
}

fn byte_list(fields: &Value, name: &str) -> anyhow::Result<Vec<Vec<u8>>> {
    fields
        .get(name)
        .and_then(Value::as_array)
        .with_context(|| format!("missing array {name}"))?
        .iter()
        .map(|value| hex(value.as_str().context("hex item is not a string")?))
        .collect()
}

fn status(value: &str) -> anyhow::Result<ResponseStatus> {
    match value {
        "success" => Ok(ResponseStatus::Success),
        "spawn_failed" => Ok(ResponseStatus::SpawnFailed),
        "cancelled" => Ok(ResponseStatus::Cancelled),
        "deadline_exceeded" => Ok(ResponseStatus::DeadlineExceeded),
        "stdout_limit_exceeded" => Ok(ResponseStatus::StdoutLimitExceeded),
        "stderr_limit_exceeded" => Ok(ResponseStatus::StderrLimitExceeded),
        "exit_failed" => Ok(ResponseStatus::ExitFailed),
        "supervision_failed" => Ok(ResponseStatus::SupervisionFailed),
        _ => anyhow::bail!("unknown fixture status"),
    }
}

fn compare(frame: &Frame<'_>, literal: &[u8], name: &str) -> anyhow::Result<()> {
    let expected = expectation(frame);
    assert_eq!(
        encode_frame(frame, expected)?,
        literal,
        "golden encoding {name}"
    );
    assert_eq!(
        decode_frame(literal, expected)?,
        *frame,
        "golden decoding {name}"
    );
    Ok(())
}

fn check_request(fields: &Value, lane: Lane, literal: &[u8], name: &str) -> anyhow::Result<()> {
    let program = bytes(fields, "program_hex")?;
    let arguments = byte_list(fields, "args_hex")?;
    let frame = Frame::Request(Request {
        lane,
        id: text(fields, "request_id")?.parse()?,
        remaining_ms: text(fields, "remaining_deadline_ms")?.parse()?,
        limits: StreamLimits {
            stdout: number(fields, "max_stdout_bytes")?,
            stderr: number(fields, "max_stderr_bytes")?,
        },
        native_identity: array(fields, "native_tool_identity_sha256_hex")?,
        program: &program,
        arguments: arguments.iter().map(Vec::as_slice).collect(),
    });
    compare(&frame, literal, name)
}

fn check_response(fields: &Value, lane: Lane, literal: &[u8], name: &str) -> anyhow::Result<()> {
    let stdout = bytes(fields, "stdout_hex")?;
    let detail = bytes(fields, "stderr_detail_hex")?;
    let evidence = byte_list(fields, "evidence_items_hex")?;
    let flags = number(fields, "flags")?;
    anyhow::ensure!(flags <= 1, "unknown fixture flags");
    let frame = Frame::Response(Response {
        lane,
        id: text(fields, "request_id")?.parse()?,
        status: status(text(fields, "status")?)?,
        stdout: &stdout,
        detail: &detail,
        evidence: evidence
            .iter()
            .map(|item| std::str::from_utf8(item))
            .collect::<Result<_, _>>()?,
        evidence_truncated: flags == 1,
    });
    compare(&frame, literal, name)
}

fn check_vector(vector: &Value) -> anyhow::Result<()> {
    let name = text(vector, "name")?;
    let literal = bytes(vector, "frame_hex")?;
    let fields = vector.get("fields").context("missing vector fields")?;
    assert_eq!(number(fields, "protocol_version")?, 1);
    let lane = match text(vector, "lane")? {
        "control" => Lane::Control,
        "job" => Lane::Job,
        _ => anyhow::bail!("unknown fixture lane"),
    };
    match text(vector, "kind")? {
        "hello" => compare(
            &Frame::Hello(Hello {
                lane,
                nonce: array(fields, "nonce_hex")?,
            }),
            &literal,
            name,
        ),
        "hello_ack" => compare(
            &Frame::HelloAck(HelloAck {
                hello: Hello {
                    lane,
                    nonce: array(fields, "nonce_hex")?,
                },
                process_id: number(fields, "child_pid")?,
                process_group_id: number(fields, "process_group_id")?,
                executable: ExecutableIdentity {
                    length: text(fields, "executable_byte_length")?.parse()?,
                    sha256: array(fields, "executable_sha256_hex")?,
                },
            }),
            &literal,
            name,
        ),
        "request" => check_request(fields, lane, &literal, name),
        "response" => check_response(fields, lane, &literal, name),
        _ => anyhow::bail!("unknown fixture kind"),
    }
}

#[test]
fn independent_literal_vectors_match_encoding_and_decoding() -> anyhow::Result<()> {
    let corpus: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/rvb1-vectors.json"
    )))?;
    assert_eq!(number(&corpus, "version")?, 1);
    let vectors = corpus
        .get("vectors")
        .and_then(Value::as_array)
        .context("missing vectors")?;
    assert_eq!(vectors.len(), 25);
    let mut names = BTreeSet::new();
    let mut cases = BTreeSet::new();
    for vector in vectors {
        assert!(names.insert(text(vector, "name")?));
        check_vector(vector)?;
        let kind = text(vector, "kind")?;
        let status = if kind == "response" {
            text(&vector["fields"], "status")?
        } else {
            ""
        };
        cases.insert((text(vector, "lane")?, kind, status));
    }
    for lane in ["control", "job"] {
        for kind in ["hello", "hello_ack", "request"] {
            assert!(cases.contains(&(lane, kind, "")));
        }
        for status in [
            "success",
            "spawn_failed",
            "cancelled",
            "deadline_exceeded",
            "stdout_limit_exceeded",
            "stderr_limit_exceeded",
            "exit_failed",
            "supervision_failed",
        ] {
            assert!(cases.contains(&(lane, "response", status)));
        }
    }
    Ok(())
}
