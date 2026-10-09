use super::*;

#[test]
fn every_header_kind_has_exact_independent_payload_bounds() -> anyhow::Result<()> {
    let executable = ExecutableIdentity {
        length: 1,
        sha256: [0; 32],
    };
    let cases = [
        (1, 20_u32, 20_u32, ExpectedFrame::Hello { lane: Lane::Job }),
        (
            2,
            68,
            68,
            ExpectedFrame::HelloAck {
                hello: hello(Lane::Job),
                process_id: 1,
                executable,
            },
        ),
        (
            16,
            69,
            1_048_576,
            ExpectedFrame::Request {
                lane: Lane::Job,
                id: 1,
            },
        ),
        (
            17,
            32,
            50_335_744,
            ExpectedFrame::Response {
                lane: Lane::Job,
                id: 1,
                limits: StreamLimits {
                    stdout: 0,
                    stderr: 0,
                },
            },
        ),
    ];
    for (kind, minimum, maximum, expected) in cases {
        let mut header = *b"RVB1\0\0\0\0\0\0\0\0";
        header[4] = kind;
        for length in [minimum, maximum] {
            header[8..].copy_from_slice(&length.to_be_bytes());
            assert_eq!(
                FrameHeader::parse(&header, expected)?.payload_length(),
                usize::try_from(length)?
            );
        }
        for length in [0, minimum - 1, maximum + 1, u32::MAX] {
            header[8..].copy_from_slice(&length.to_be_bytes());
            assert_eq!(
                FrameHeader::parse(&header, expected),
                Err(ProtocolError::LimitExceeded)
            );
        }
    }
    assert_eq!(
        validate::sum(usize::MAX, 1),
        Err(ProtocolError::LimitExceeded)
    );
    assert_eq!(validate::sum(usize::MAX, 0)?, usize::MAX);
    Ok(())
}

#[test]
fn request_program_and_stream_boundaries_are_exact() -> anyhow::Result<()> {
    let mut program = vec![b'/'; 4096];
    let mut value = request(Lane::Control);
    value.program = &program;
    roundtrip(&Frame::Request(value))?;
    program.push(b'/');
    let mut value = request(Lane::Control);
    value.program = &program;
    let frame = Frame::Request(value);
    assert_eq!(
        encode_frame(&frame, expectation(&frame)),
        Err(ProtocolError::LimitExceeded)
    );
    for limits in [
        StreamLimits {
            stdout: 16_777_217,
            stderr: 0,
        },
        StreamLimits {
            stdout: 0,
            stderr: 16_777_217,
        },
    ] {
        let mut value = request(Lane::Job);
        value.limits = limits;
        let frame = Frame::Request(value);
        assert_eq!(
            encode_frame(&frame, expectation(&frame)),
            Err(ProtocolError::LimitExceeded)
        );
    }
    let mut value = request(Lane::Job);
    value.program = b"/\xff";
    value.arguments.clear();
    value.limits = StreamLimits {
        stdout: 0,
        stderr: 0,
    };
    roundtrip(&Frame::Request(value))?;
    Ok(())
}

#[test]
fn maximal_streams_and_evidence_are_accepted_but_one_more_byte_is_not() -> anyhow::Result<()> {
    let output = vec![0xff; 16_777_217];
    let mut value = response(Lane::Job, ResponseStatus::Success);
    value.stdout = &output[..16_777_216];
    value.detail = &output[..16_777_216];
    roundtrip(&Frame::Response(value.clone()))?;
    value.stdout = &output;
    let frame = Frame::Response(value);
    assert_eq!(
        encode_frame(&frame, expectation(&frame)),
        Err(ProtocolError::LimitExceeded)
    );
    let evidence = "e".repeat(16_777_209);
    let mut value = response(Lane::Job, ResponseStatus::ExitFailed);
    value.detail = &output[..16_777_216];
    value.evidence = vec![&evidence[..16_777_208]];
    roundtrip(&Frame::Response(value.clone()))?;
    value.evidence = vec![&evidence];
    let frame = Frame::Response(value);
    assert_eq!(
        encode_frame(&frame, expectation(&frame)),
        Err(ProtocolError::LimitExceeded)
    );
    Ok(())
}

#[test]
fn invalid_lanes_ids_and_request_values_are_rejected_on_decode() -> anyhow::Result<()> {
    let frame = Frame::Request(request(Lane::Job));
    let expected = expectation(&frame);
    let bytes = encode_frame(&frame, expected)?;
    for lane in 0..=255 {
        if lane == 2 {
            continue;
        }
        let mut altered = bytes.clone();
        altered[14] = lane;
        assert!(decode_frame(&altered, expected).is_err());
    }
    for (start, end) in [(16, 24), (24, 32)] {
        let mut altered = bytes.clone();
        altered[start..end].fill(0);
        assert!(decode_frame(&altered, expected).is_err());
    }
    for offset in [15, 76, 97] {
        let mut altered = bytes.clone();
        altered[offset] = u8::from(offset == 15);
        assert!(decode_frame(&altered, expected).is_err());
    }
    let mut wrong_id = bytes;
    wrong_id[23] = 2;
    assert_eq!(
        decode_frame(&wrong_id, expected),
        Err(ProtocolError::IdentityMismatch)
    );
    assert!(encode_frame(&frame, ExpectedFrame::Hello { lane: Lane::Job }).is_err());
    Ok(())
}

#[test]
fn evidence_noncanonical_counts_lengths_and_order_are_rejected_on_decode() -> anyhow::Result<()> {
    let mut value = response(Lane::Job, ResponseStatus::SupervisionFailed);
    value.evidence = vec!["a", "b"];
    let frame = Frame::Response(value);
    let expected = expectation(&frame);
    let bytes = encode_frame(&frame, expected)?;
    for (offset, replacement) in [
        (43, 0),
        (43, 1),
        (43, 3),
        (47, 0),
        (47, 2),
        (48, b'b'),
        (53, b'a'),
        (36, 1),
        (15, 0),
    ] {
        let mut altered = bytes.clone();
        altered[offset] = replacement;
        assert!(
            decode_frame(&altered, expected).is_err(),
            "accepted mutation at {offset}"
        );
    }
    let mut value = response(Lane::Control, ResponseStatus::ExitFailed);
    value.evidence = vec!["a", "b", "\u{e9}"];
    roundtrip(&Frame::Response(value))?;
    Ok(())
}

#[test]
fn generated_mutations_with_valid_headers_only_accept_canonical_frames() -> anyhow::Result<()> {
    let mut failure = response(Lane::Job, ResponseStatus::ExitFailed);
    failure.evidence = vec!["cleanup", "recovery"];
    for frame in [
        Frame::Hello(hello(Lane::Job)),
        Frame::Request(request(Lane::Job)),
        Frame::Response(failure),
    ] {
        let expected = expectation(&frame);
        let bytes = encode_frame(&frame, expected)?;
        for offset in 12..bytes.len() {
            for mask in [1, 0x80, 0xff] {
                let mut altered = bytes.clone();
                altered[offset] ^= mask;
                if let Ok(decoded) = decode_frame(&altered, expected) {
                    assert_eq!(encode_frame(&decoded, expected)?, altered);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn evidence_preflight_rejects_legal_counts_with_invalid_items_without_reservation()
-> anyhow::Result<()> {
    use super::super::decode::evidence_count;

    let mut bytes = vec![0; 16_777_214];
    bytes[..4].copy_from_slice(&3_355_442_u32.to_be_bytes());
    for length in [16_777_209_u32, u32::MAX] {
        bytes[4..8].copy_from_slice(&length.to_be_bytes());
        assert_eq!(evidence_count(&bytes), Err(ProtocolError::LimitExceeded));
    }
    bytes[4..8].copy_from_slice(&16_777_208_u32.to_be_bytes());
    assert_eq!(evidence_count(&bytes), Err(ProtocolError::Truncated));
    bytes[4..8].fill(0);
    assert_eq!(evidence_count(&bytes), Err(ProtocolError::Malformed));
    assert_eq!(evidence_count(&[0, 0, 0, 0])?, 0);
    let valid = [0, 0, 0, 2, 0, 0, 0, 1, b'a', 0, 0, 0, 1, b'b'];
    assert_eq!(evidence_count(&valid)?, 2);
    for offset in [8, 13] {
        let mut invalid = valid;
        invalid[offset] = 0xff;
        assert_eq!(evidence_count(&invalid), Err(ProtocolError::Malformed));
    }
    Ok(())
}

#[test]
fn decoded_responses_obey_original_request_limits_including_zero() -> anyhow::Result<()> {
    for status in [ResponseStatus::Success, ResponseStatus::ExitFailed] {
        let mut value = response(Lane::Job, status);
        value.detail = b"detail";
        if status == ResponseStatus::Success {
            value.stdout = b"out";
        }
        let frame = Frame::Response(value);
        let bytes = encode_frame(&frame, expectation(&frame))?;
        for limits in [
            StreamLimits {
                stdout: 0,
                stderr: 0,
            },
            StreamLimits {
                stdout: 3,
                stderr: 5,
            },
        ] {
            let expected = ExpectedFrame::Response {
                lane: Lane::Job,
                id: 1,
                limits,
            };
            assert_eq!(
                decode_frame(&bytes, expected),
                Err(ProtocolError::LimitExceeded)
            );
        }
        let exact = ExpectedFrame::Response {
            lane: Lane::Job,
            id: 1,
            limits: StreamLimits {
                stdout: 3,
                stderr: 6,
            },
        };
        assert_eq!(decode_frame(&bytes, exact)?, frame);
        if status == ResponseStatus::Success {
            let shorter = ExpectedFrame::Response {
                lane: Lane::Job,
                id: 1,
                limits: StreamLimits {
                    stdout: 2,
                    stderr: 6,
                },
            };
            assert_eq!(
                decode_frame(&bytes, shorter),
                Err(ProtocolError::LimitExceeded)
            );
        }
    }
    Ok(())
}
