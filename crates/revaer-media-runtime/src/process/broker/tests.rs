use super::*;

mod bounds;
mod vectors;

fn hello(lane: Lane) -> Hello {
    Hello {
        lane,
        nonce: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    }
}

fn request(lane: Lane) -> Request<'static> {
    Request {
        lane,
        id: 1,
        remaining_ms: 1,
        limits: StreamLimits {
            stdout: 16_777_216,
            stderr: 16_777_216,
        },
        native_identity: [3; 32],
        program: b"/bin/tool",
        arguments: vec![b"", b"\xffarg"],
    }
}

fn response(lane: Lane, status: ResponseStatus) -> Response<'static> {
    Response {
        lane,
        id: 1,
        status,
        stdout: b"",
        detail: b"",
        evidence: Vec::new(),
        evidence_truncated: false,
    }
}

fn expectation(frame: &Frame<'_>) -> ExpectedFrame {
    match frame {
        Frame::Hello(value) => ExpectedFrame::Hello { lane: value.lane },
        Frame::HelloAck(value) => ExpectedFrame::HelloAck {
            hello: value.hello,
            process_id: value.process_id,
            executable: value.executable,
        },
        Frame::Request(value) => ExpectedFrame::Request {
            lane: value.lane,
            id: value.id,
        },
        Frame::Response(value) => ExpectedFrame::Response {
            lane: value.lane,
            id: value.id,
            limits: StreamLimits {
                stdout: 16_777_216,
                stderr: 16_777_216,
            },
        },
    }
}

fn roundtrip(frame: &Frame<'_>) -> anyhow::Result<Vec<u8>> {
    let expected = expectation(frame);
    let bytes = encode_frame(frame, expected)?;
    assert_eq!(decode_frame(&bytes, expected)?, *frame);
    assert_eq!(
        encode_frame(&decode_frame(&bytes, expected)?, expected)?,
        bytes
    );
    Ok(bytes)
}

#[test]
fn all_kinds_lanes_and_statuses_roundtrip_without_text_conversion() -> anyhow::Result<()> {
    for lane in [Lane::Control, Lane::Job] {
        let greeting = hello(lane);
        let hello_bytes = roundtrip(&Frame::Hello(greeting))?;
        assert_eq!(
            &hello_bytes[..12],
            &[82, 86, 66, 49, 1, 0, 0, 0, 0, 0, 0, 20]
        );
        assert_eq!(&hello_bytes[16..], &greeting.nonce);
        roundtrip(&Frame::HelloAck(HelloAck {
            hello: greeting,
            process_id: 42,
            process_group_id: 42,
            executable: ExecutableIdentity {
                length: u64::MAX,
                sha256: [9; 32],
            },
        }))?;
        roundtrip(&Frame::Request(request(lane)))?;
        for status in [
            ResponseStatus::Success,
            ResponseStatus::SpawnFailed,
            ResponseStatus::Cancelled,
            ResponseStatus::DeadlineExceeded,
            ResponseStatus::StdoutLimitExceeded,
            ResponseStatus::StderrLimitExceeded,
            ResponseStatus::ExitFailed,
            ResponseStatus::SupervisionFailed,
        ] {
            roundtrip(&Frame::Response(response(lane, status)))?;
        }
    }
    let mut value = response(Lane::Job, ResponseStatus::Success);
    value.stdout = b"\x00\xff";
    value.detail = b"\xfe\x00";
    roundtrip(&Frame::Response(value))?;
    Ok(())
}

#[test]
fn every_truncation_trailer_and_expected_kind_mismatch_fails() -> anyhow::Result<()> {
    let frames = [
        Frame::Hello(hello(Lane::Control)),
        Frame::HelloAck(HelloAck {
            hello: hello(Lane::Control),
            process_id: 1,
            process_group_id: 1,
            executable: ExecutableIdentity {
                length: 0,
                sha256: [0; 32],
            },
        }),
        Frame::Request(request(Lane::Job)),
        Frame::Response(response(Lane::Job, ResponseStatus::Success)),
    ];
    for frame in frames {
        let expected = expectation(&frame);
        let mut bytes = encode_frame(&frame, expected)?;
        for end in 0..bytes.len() {
            assert!(
                decode_frame(&bytes[..end], expected).is_err(),
                "accepted truncation at {end}"
            );
        }
        bytes.push(0);
        assert_eq!(
            decode_frame(&bytes, expected),
            Err(ProtocolError::Malformed)
        );
        assert!(
            decode_frame(
                &bytes,
                ExpectedFrame::Hello {
                    lane: Lane::Control
                }
            )
            .is_err()
        );
    }
    Ok(())
}

#[test]
fn headers_fail_before_payload_allocation_and_unknown_registry_is_rejected() -> anyhow::Result<()> {
    let expected = ExpectedFrame::Request {
        lane: Lane::Job,
        id: 1,
    };
    let mut header = *b"RVB1\x10\0\0\0\0\0\0\x45";
    assert_eq!(FrameHeader::parse(&header, expected)?.payload_length(), 69);
    for length in [0_u32, 68, 1_048_577, u32::MAX] {
        header[8..12].copy_from_slice(&length.to_be_bytes());
        assert_eq!(
            FrameHeader::parse(&header, expected),
            Err(ProtocolError::LimitExceeded)
        );
    }
    header[8..12].copy_from_slice(&69_u32.to_be_bytes());
    for offset in [0, 1, 2, 3, 5, 6, 7] {
        let mut malformed = header;
        malformed[offset] ^= 1;
        assert_eq!(
            FrameHeader::parse(&malformed, expected),
            Err(ProtocolError::Malformed)
        );
    }
    for kind in 0..=255 {
        if [1, 2, 0x10, 0x11].contains(&kind) {
            continue;
        }
        header[4] = kind;
        assert_eq!(
            FrameHeader::parse(&header, expected),
            Err(ProtocolError::Malformed)
        );
    }
    Ok(())
}

#[test]
fn handshake_rejects_all_identity_mismatches_and_non_linux_pids() -> anyhow::Result<()> {
    let ack = HelloAck {
        hello: hello(Lane::Job),
        process_id: 1234,
        process_group_id: 1234,
        executable: ExecutableIdentity {
            length: 10,
            sha256: [7; 32],
        },
    };
    let frame = Frame::HelloAck(ack);
    let expected = expectation(&frame);
    let bytes = encode_frame(&frame, expected)?;
    for offset in [14, 16, 31, 35, 39, 47, 48, 79] {
        let mut malformed = bytes.clone();
        malformed[offset] ^= 1;
        assert!(
            decode_frame(&malformed, expected).is_err(),
            "accepted altered identity at {offset}"
        );
    }
    for pid in [0, 2_147_483_648, u32::MAX] {
        let invalid = Frame::HelloAck(HelloAck {
            process_id: pid,
            process_group_id: pid,
            ..ack
        });
        assert!(encode_frame(&invalid, expected).is_err());
    }
    let mut malformed = bytes.clone();
    malformed[12] = 1;
    assert_eq!(
        decode_frame(&malformed, expected),
        Err(ProtocolError::Malformed)
    );
    malformed = bytes;
    malformed[15] = 1;
    assert_eq!(
        decode_frame(&malformed, expected),
        Err(ProtocolError::Malformed)
    );
    Ok(())
}

#[test]
fn requests_enforce_lane_timeouts_ids_paths_and_string_bounds() -> anyhow::Result<()> {
    for (lane, maximum) in [(Lane::Control, 30_000), (Lane::Job, 86_400_000)] {
        let mut value = request(lane);
        value.remaining_ms = maximum;
        value.id = u64::MAX;
        roundtrip(&Frame::Request(value.clone()))?;
        for remaining_ms in [0, maximum + 1, u64::MAX] {
            value.remaining_ms = remaining_ms;
            let frame = Frame::Request(value.clone());
            assert!(encode_frame(&frame, expectation(&frame)).is_err());
        }
    }
    for program in [b"".as_slice(), b"relative", b"/a\0b"] {
        let mut value = request(Lane::Job);
        value.program = program;
        let frame = Frame::Request(value);
        assert!(encode_frame(&frame, expectation(&frame)).is_err());
    }
    let long = vec![b'a'; 4_097];
    let mut value = request(Lane::Job);
    value.arguments = vec![&long];
    let frame = Frame::Request(value);
    assert_eq!(
        encode_frame(&frame, expectation(&frame)),
        Err(ProtocolError::LimitExceeded)
    );
    let mut value = request(Lane::Job);
    value.arguments = vec![b"\0"];
    let frame = Frame::Request(value);
    assert_eq!(
        encode_frame(&frame, expectation(&frame)),
        Err(ProtocolError::Malformed)
    );
    Ok(())
}

#[test]
fn request_argument_count_and_combined_byte_bound_are_exact() -> anyhow::Result<()> {
    let mut value = request(Lane::Job);
    value.program = b"/";
    value.arguments = vec![b""; 4096];
    roundtrip(&Frame::Request(value.clone()))?;
    value.arguments.push(b"");
    let frame = Frame::Request(value);
    assert_eq!(
        encode_frame(&frame, expectation(&frame)),
        Err(ProtocolError::LimitExceeded)
    );
    let block = vec![b'x'; 4_096];
    let tail = vec![b'y'; 575];
    let mut value = request(Lane::Job);
    value.program = b"/";
    value.arguments = vec![&block; 244];
    value.arguments.push(&tail);
    roundtrip(&Frame::Request(value.clone()))?;
    value.arguments.push(b"x");
    let frame = Frame::Request(value);
    assert_eq!(
        encode_frame(&frame, expectation(&frame)),
        Err(ProtocolError::LimitExceeded)
    );
    Ok(())
}

#[test]
fn response_limits_status_consistency_and_evidence_canonicality_are_enforced() -> anyhow::Result<()>
{
    let expected = ExpectedFrame::Response {
        lane: Lane::Job,
        id: 1,
        limits: StreamLimits {
            stdout: 0,
            stderr: 0,
        },
    };
    let mut value = response(Lane::Job, ResponseStatus::Success);
    encode_frame(&Frame::Response(value.clone()), expected)?;
    value.stdout = b"x";
    assert_eq!(
        encode_frame(&Frame::Response(value.clone()), expected),
        Err(ProtocolError::LimitExceeded)
    );
    value.stdout = b"";
    value.detail = b"x";
    assert_eq!(
        encode_frame(&Frame::Response(value.clone()), expected),
        Err(ProtocolError::LimitExceeded)
    );
    value.detail = b"";
    value.evidence_truncated = true;
    assert_eq!(
        encode_frame(&Frame::Response(value.clone()), expected),
        Err(ProtocolError::Malformed)
    );
    value.status = ResponseStatus::ExitFailed;
    value.evidence = vec!["alpha", "zeta"];
    roundtrip(&Frame::Response(value.clone()))?;
    for evidence in [vec![""], vec!["same", "same"], vec!["zeta", "alpha"]] {
        value.evidence = evidence;
        let frame = Frame::Response(value.clone());
        assert_eq!(
            encode_frame(&frame, expectation(&frame)),
            Err(ProtocolError::Malformed)
        );
    }
    value.evidence.clear();
    value.stdout = b"failure output";
    let frame = Frame::Response(value);
    assert_eq!(
        encode_frame(&frame, expectation(&frame)),
        Err(ProtocolError::Malformed)
    );
    Ok(())
}

#[test]
fn decoder_rejects_malformed_field_counts_flags_lengths_and_utf8() -> anyhow::Result<()> {
    let mut value = response(Lane::Job, ResponseStatus::ExitFailed);
    value.evidence = vec!["a"];
    let frame = Frame::Response(value);
    let expected = expectation(&frame);
    let bytes = encode_frame(&frame, expected)?;
    for (offset, replacement) in [(15, 8), (24, 2), (25, 1), (26, 1), (27, 1), (48, 0xff)] {
        let mut malformed = bytes.clone();
        malformed[offset] = replacement;
        assert!(
            decode_frame(&malformed, expected).is_err(),
            "accepted malformed response offset {offset}"
        );
    }
    for offset in [28, 32, 36, 40, 44] {
        let mut malformed = bytes.clone();
        malformed[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(decode_frame(&malformed, expected).is_err());
    }
    let frame = Frame::Request(request(Lane::Job));
    let expected = expectation(&frame);
    let bytes = encode_frame(&frame, expected)?;
    for offset in [72, 85] {
        let mut malformed = bytes.clone();
        malformed[offset..offset + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(decode_frame(&malformed, expected).is_err());
    }
    Ok(())
}

#[test]
fn invalid_caller_expectations_are_not_accepted_as_remote_authority() {
    for expected in [
        ExpectedFrame::Request {
            lane: Lane::Job,
            id: 0,
        },
        ExpectedFrame::Response {
            lane: Lane::Job,
            id: 1,
            limits: StreamLimits {
                stdout: u32::MAX,
                stderr: 0,
            },
        },
    ] {
        assert!(encode_frame(&Frame::Request(request(Lane::Job)), expected).is_err());
    }
}

#[test]
fn deterministic_arbitrary_bytes_never_escape_the_fallible_parser() -> anyhow::Result<()> {
    let expected = ExpectedFrame::Request {
        lane: Lane::Job,
        id: 1,
    };
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    for length in 0..256 {
        let mut bytes = vec![0; length];
        for byte in &mut bytes {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *byte = state.to_le_bytes()[0];
        }
        if let Ok(frame) = decode_frame(&bytes, expected) {
            assert_eq!(encode_frame(&frame, expected)?, bytes);
        }
    }
    Ok(())
}

#[test]
fn frame_and_error_debug_output_never_contains_payloads() {
    let frame = Frame::Request(request(Lane::Job));
    assert_eq!(format!("{frame:?}"), "BrokerFrame(Request)");
    assert_eq!(
        ProtocolError::IdentityMismatch.to_string(),
        "broker frame identity does not match"
    );
}
