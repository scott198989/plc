use std::io::{Cursor, Read, Write};

use crate::{
    BackingAttestation, BrokerError, BrokerErrorCode, BrokerRequest, BrokerResponse,
    FixedFileSystem, GrantId, MAX_PROJECT_BYTES, RequestEnvelope, ResponseEnvelope,
};

const REQUEST_MAGIC: [u8; 8] = *b"P2VLABQ1";
const RESPONSE_MAGIC: [u8; 8] = *b"P2VLABR1";
const HEADER_BYTES: usize = 24;
const MAX_NAME_BYTES: usize = 255;
const MAX_FRAME_PAYLOAD: usize = MAX_PROJECT_BYTES + MAX_NAME_BYTES + 16;

const REQUEST_HANDSHAKE: u8 = 0;
const REQUEST_LIST: u8 = 1;
const REQUEST_OPEN: u8 = 2;
const REQUEST_SAVE_AS: u8 = 3;
const REQUEST_SAVE: u8 = 4;
const REQUEST_REVOKE: u8 = 5;

const RESPONSE_ERROR: u8 = 0;
const RESPONSE_HANDSHAKE: u8 = 1;
const RESPONSE_PROJECTS: u8 = 2;
const RESPONSE_OPENED: u8 = 3;
const RESPONSE_SAVED: u8 = 4;
const RESPONSE_REVOKED: u8 = 5;

pub fn read_request(reader: &mut impl Read) -> Result<Option<RequestEnvelope>, BrokerError> {
    let Some(header) = read_header(reader, REQUEST_MAGIC)? else {
        return Ok(None);
    };
    let payload = read_payload(reader, header.payload_len)?;
    let mut cursor = Cursor::new(payload.as_slice());
    let request = match header.tag {
        REQUEST_HANDSHAKE => {
            require_empty(&cursor)?;
            BrokerRequest::Handshake
        }
        REQUEST_LIST => {
            require_empty(&cursor)?;
            BrokerRequest::ListProjects
        }
        REQUEST_OPEN => BrokerRequest::Open {
            name: read_name(&mut cursor)?,
        },
        REQUEST_SAVE_AS => BrokerRequest::SaveAs {
            name: read_name(&mut cursor)?,
            bytes: read_bytes(&mut cursor)?,
        },
        REQUEST_SAVE => BrokerRequest::Save {
            grant_id: GrantId::from_wire(read_u64(&mut cursor)?),
            bytes: read_bytes(&mut cursor)?,
        },
        REQUEST_REVOKE => BrokerRequest::Revoke {
            grant_id: GrantId::from_wire(read_u64(&mut cursor)?),
        },
        _ => return Err(invalid_frame()),
    };
    require_consumed(&cursor)?;
    Ok(Some(RequestEnvelope {
        protocol_version: header.version,
        request_id: header.request_id,
        request,
    }))
}

pub fn write_response(
    writer: &mut impl Write,
    envelope: &ResponseEnvelope,
) -> Result<(), BrokerError> {
    let (tag, payload) = encode_response_payload(&envelope.response)?;
    write_header(
        writer,
        RESPONSE_MAGIC,
        envelope.protocol_version,
        tag,
        envelope.request_id,
        payload.len(),
    )?;
    writer
        .write_all(&payload)
        .and_then(|()| writer.flush())
        .map_err(|_| protocol_io_error())
}

pub fn encode_request(envelope: &RequestEnvelope) -> Result<Vec<u8>, BrokerError> {
    let (tag, payload) = encode_request_payload(&envelope.request)?;
    let mut frame = Vec::with_capacity(HEADER_BYTES + payload.len());
    write_header(
        &mut frame,
        REQUEST_MAGIC,
        envelope.protocol_version,
        tag,
        envelope.request_id,
        payload.len(),
    )?;
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_response(reader: &mut impl Read) -> Result<Option<ResponseEnvelope>, BrokerError> {
    let Some(header) = read_header(reader, RESPONSE_MAGIC)? else {
        return Ok(None);
    };
    let payload = read_payload(reader, header.payload_len)?;
    let mut cursor = Cursor::new(payload.as_slice());
    let response = match header.tag {
        RESPONSE_ERROR => Err(decode_error(&mut cursor)?),
        RESPONSE_HANDSHAKE => Ok(BrokerResponse::Handshake {
            attestation: decode_attestation(&mut cursor)?,
        }),
        RESPONSE_PROJECTS => {
            let count = usize::from(read_u16(&mut cursor)?);
            if count > 4096 {
                return Err(invalid_frame());
            }
            let mut names = Vec::with_capacity(count);
            for _ in 0..count {
                names.push(read_name(&mut cursor)?);
            }
            Ok(BrokerResponse::Projects { names })
        }
        RESPONSE_OPENED => Ok(BrokerResponse::Opened {
            display_name: read_name(&mut cursor)?,
            grant_id: GrantId::from_wire(read_u64(&mut cursor)?),
            bytes: read_bytes(&mut cursor)?,
        }),
        RESPONSE_SAVED => Ok(BrokerResponse::Saved {
            display_name: read_name(&mut cursor)?,
            grant_id: GrantId::from_wire(read_u64(&mut cursor)?),
            verified_bytes: usize::try_from(read_u64(&mut cursor)?).map_err(|_| invalid_frame())?,
        }),
        RESPONSE_REVOKED => Ok(BrokerResponse::Revoked),
        _ => return Err(invalid_frame()),
    };
    require_consumed(&cursor)?;
    Ok(Some(ResponseEnvelope {
        protocol_version: header.version,
        request_id: header.request_id,
        response,
    }))
}

fn encode_request_payload(request: &BrokerRequest) -> Result<(u8, Vec<u8>), BrokerError> {
    let mut payload = Vec::new();
    let tag = match request {
        BrokerRequest::Handshake => REQUEST_HANDSHAKE,
        BrokerRequest::ListProjects => REQUEST_LIST,
        BrokerRequest::Open { name } => {
            write_name(&mut payload, name)?;
            REQUEST_OPEN
        }
        BrokerRequest::SaveAs { name, bytes } => {
            write_name(&mut payload, name)?;
            write_bytes(&mut payload, bytes)?;
            REQUEST_SAVE_AS
        }
        BrokerRequest::Save { grant_id, bytes } => {
            payload.extend_from_slice(&grant_id.to_wire().to_le_bytes());
            write_bytes(&mut payload, bytes)?;
            REQUEST_SAVE
        }
        BrokerRequest::Revoke { grant_id } => {
            payload.extend_from_slice(&grant_id.to_wire().to_le_bytes());
            REQUEST_REVOKE
        }
    };
    Ok((tag, payload))
}

fn encode_response_payload(
    response: &Result<BrokerResponse, BrokerError>,
) -> Result<(u8, Vec<u8>), BrokerError> {
    let mut payload = Vec::new();
    let tag = match response {
        Err(error) => {
            payload.extend_from_slice(&error_code_to_u16(error.code).to_le_bytes());
            write_short_text(&mut payload, error.message)?;
            RESPONSE_ERROR
        }
        Ok(BrokerResponse::Handshake { attestation }) => {
            encode_attestation(&mut payload, *attestation);
            RESPONSE_HANDSHAKE
        }
        Ok(BrokerResponse::Projects { names }) => {
            let count = u16::try_from(names.len()).map_err(|_| invalid_frame())?;
            payload.extend_from_slice(&count.to_le_bytes());
            for name in names {
                write_name(&mut payload, name)?;
            }
            RESPONSE_PROJECTS
        }
        Ok(BrokerResponse::Opened {
            display_name,
            grant_id,
            bytes,
        }) => {
            write_name(&mut payload, display_name)?;
            payload.extend_from_slice(&grant_id.to_wire().to_le_bytes());
            write_bytes(&mut payload, bytes)?;
            RESPONSE_OPENED
        }
        Ok(BrokerResponse::Saved {
            display_name,
            grant_id,
            verified_bytes,
        }) => {
            write_name(&mut payload, display_name)?;
            payload.extend_from_slice(&grant_id.to_wire().to_le_bytes());
            payload.extend_from_slice(
                &u64::try_from(*verified_bytes)
                    .map_err(|_| invalid_frame())?
                    .to_le_bytes(),
            );
            RESPONSE_SAVED
        }
        Ok(BrokerResponse::Revoked) => RESPONSE_REVOKED,
    };
    if payload.len() > MAX_FRAME_PAYLOAD {
        return Err(invalid_frame());
    }
    Ok((tag, payload))
}

struct Header {
    version: u16,
    tag: u8,
    request_id: u64,
    payload_len: usize,
}

fn read_header(reader: &mut impl Read, magic: [u8; 8]) -> Result<Option<Header>, BrokerError> {
    let mut header = [0_u8; HEADER_BYTES];
    match reader.read(&mut header[..1]) {
        Ok(0) => return Ok(None),
        Ok(1) => {}
        Ok(_) => unreachable!(),
        Err(_) => return Err(protocol_io_error()),
    }
    reader
        .read_exact(&mut header[1..])
        .map_err(|_| invalid_frame())?;
    if header[..8] != magic || header[11] != 0 {
        return Err(invalid_frame());
    }
    let version = u16::from_le_bytes([header[8], header[9]]);
    let tag = header[10];
    let request_id = u64::from_le_bytes(header[12..20].try_into().unwrap());
    let payload_len = usize::try_from(u32::from_le_bytes(header[20..24].try_into().unwrap()))
        .map_err(|_| invalid_frame())?;
    if payload_len > MAX_FRAME_PAYLOAD {
        return Err(invalid_frame());
    }
    Ok(Some(Header {
        version,
        tag,
        request_id,
        payload_len,
    }))
}

fn write_header(
    writer: &mut impl Write,
    magic: [u8; 8],
    version: u16,
    tag: u8,
    request_id: u64,
    payload_len: usize,
) -> Result<(), BrokerError> {
    let payload_len = u32::try_from(payload_len).map_err(|_| invalid_frame())?;
    let mut header = [0_u8; HEADER_BYTES];
    header[..8].copy_from_slice(&magic);
    header[8..10].copy_from_slice(&version.to_le_bytes());
    header[10] = tag;
    header[12..20].copy_from_slice(&request_id.to_le_bytes());
    header[20..24].copy_from_slice(&payload_len.to_le_bytes());
    writer.write_all(&header).map_err(|_| protocol_io_error())
}

fn read_payload(reader: &mut impl Read, length: usize) -> Result<Vec<u8>, BrokerError> {
    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .map_err(|_| invalid_frame())?;
    Ok(payload)
}

fn write_name(output: &mut Vec<u8>, value: &str) -> Result<(), BrokerError> {
    if value.len() > MAX_NAME_BYTES {
        return Err(invalid_frame());
    }
    let length = u16::try_from(value.len()).map_err(|_| invalid_frame())?;
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_name(cursor: &mut Cursor<&[u8]>) -> Result<String, BrokerError> {
    let length = usize::from(read_u16(cursor)?);
    if length == 0 || length > MAX_NAME_BYTES {
        return Err(invalid_frame());
    }
    let mut bytes = vec![0_u8; length];
    cursor.read_exact(&mut bytes).map_err(|_| invalid_frame())?;
    String::from_utf8(bytes).map_err(|_| invalid_frame())
}

fn write_short_text(output: &mut Vec<u8>, value: &str) -> Result<(), BrokerError> {
    if !value.is_ascii() || value.len() > u16::MAX.into() {
        return Err(invalid_frame());
    }
    let length = u16::try_from(value.len()).map_err(|_| invalid_frame())?;
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_short_text(cursor: &mut Cursor<&[u8]>) -> Result<&'static str, BrokerError> {
    let length = usize::from(read_u16(cursor)?);
    let mut bytes = vec![0_u8; length];
    cursor.read_exact(&mut bytes).map_err(|_| invalid_frame())?;
    let text = String::from_utf8(bytes).map_err(|_| invalid_frame())?;
    Ok(match text.as_str() {
        "The native project broker protocol version is unsupported." => {
            "The native project broker protocol version is unsupported."
        }
        "The project file grant is no longer active." => {
            "The project file grant is no longer active."
        }
        "Project files must use the .vlabproj extension." => {
            "Project files must use the .vlabproj extension."
        }
        "A project name must be one bounded ASCII file name, never a path or host target." => {
            "A project name must be one bounded ASCII file name, never a path or host target."
        }
        "Project payloads must be between 1 byte and 32 MiB." => {
            "Project payloads must be between 1 byte and 32 MiB."
        }
        _ => "The native project-file operation failed closed.",
    })
}

fn write_bytes(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), BrokerError> {
    if bytes.len() > MAX_PROJECT_BYTES {
        return Err(invalid_frame());
    }
    let length = u32::try_from(bytes.len()).map_err(|_| invalid_frame())?;
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(bytes);
    Ok(())
}

fn read_bytes(cursor: &mut Cursor<&[u8]>) -> Result<Vec<u8>, BrokerError> {
    let length = usize::try_from(read_u32(cursor)?).map_err(|_| invalid_frame())?;
    if length > MAX_PROJECT_BYTES {
        return Err(invalid_frame());
    }
    let mut bytes = vec![0_u8; length];
    cursor.read_exact(&mut bytes).map_err(|_| invalid_frame())?;
    Ok(bytes)
}

fn read_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16, BrokerError> {
    let mut bytes = [0_u8; 2];
    cursor.read_exact(&mut bytes).map_err(|_| invalid_frame())?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, BrokerError> {
    let mut bytes = [0_u8; 4];
    cursor.read_exact(&mut bytes).map_err(|_| invalid_frame())?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(cursor: &mut Cursor<&[u8]>) -> Result<u64, BrokerError> {
    let mut bytes = [0_u8; 8];
    cursor.read_exact(&mut bytes).map_err(|_| invalid_frame())?;
    Ok(u64::from_le_bytes(bytes))
}

fn require_empty(cursor: &Cursor<&[u8]>) -> Result<(), BrokerError> {
    require_consumed(cursor)
}

fn require_consumed(cursor: &Cursor<&[u8]>) -> Result<(), BrokerError> {
    if usize::try_from(cursor.position()).ok() == Some(cursor.get_ref().len()) {
        Ok(())
    } else {
        Err(invalid_frame())
    }
}

fn encode_attestation(output: &mut Vec<u8>, value: BackingAttestation) {
    output.extend_from_slice(&value.protocol_version.to_le_bytes());
    output.push(match value.file_system {
        FixedFileSystem::Ntfs => 1,
        FixedFileSystem::Refs => 2,
    });
    output.extend_from_slice(&value.volume_serial.to_le_bytes());
    for flag in [
        value.fixed_drive,
        value.native_local,
        value.provider_backed,
        value.redirected,
        value.removable,
        value.special,
    ] {
        output.push(u8::from(flag));
    }
}

fn decode_attestation(cursor: &mut Cursor<&[u8]>) -> Result<BackingAttestation, BrokerError> {
    let protocol_version = read_u16(cursor)?;
    let mut fs = [0_u8; 1];
    cursor.read_exact(&mut fs).map_err(|_| invalid_frame())?;
    let file_system = match fs[0] {
        1 => FixedFileSystem::Ntfs,
        2 => FixedFileSystem::Refs,
        _ => return Err(invalid_frame()),
    };
    let volume_serial = read_u64(cursor)?;
    let mut flags = [0_u8; 6];
    cursor.read_exact(&mut flags).map_err(|_| invalid_frame())?;
    if flags.iter().any(|flag| *flag > 1) {
        return Err(invalid_frame());
    }
    Ok(BackingAttestation {
        protocol_version,
        file_system,
        volume_serial,
        fixed_drive: flags[0] == 1,
        native_local: flags[1] == 1,
        provider_backed: flags[2] == 1,
        redirected: flags[3] == 1,
        removable: flags[4] == 1,
        special: flags[5] == 1,
    })
}

fn decode_error(cursor: &mut Cursor<&[u8]>) -> Result<BrokerError, BrokerError> {
    let code = u16_to_error_code(read_u16(cursor)?)?;
    let message = read_short_text(cursor)?;
    Ok(BrokerError { code, message })
}

fn error_code_to_u16(value: BrokerErrorCode) -> u16 {
    match value {
        BrokerErrorCode::AccessUnavailable => 1,
        BrokerErrorCode::AttestationFailed => 2,
        BrokerErrorCode::InvalidExtension => 3,
        BrokerErrorCode::InvalidFileName => 4,
        BrokerErrorCode::InvalidFrame => 5,
        BrokerErrorCode::ProjectTooLarge => 6,
        BrokerErrorCode::ProtocolMismatch => 7,
        BrokerErrorCode::ReadFailed => 8,
        BrokerErrorCode::StaleGrant => 9,
        BrokerErrorCode::UnknownGrant => 10,
        BrokerErrorCode::WriteFailed => 11,
    }
}

fn u16_to_error_code(value: u16) -> Result<BrokerErrorCode, BrokerError> {
    match value {
        1 => Ok(BrokerErrorCode::AccessUnavailable),
        2 => Ok(BrokerErrorCode::AttestationFailed),
        3 => Ok(BrokerErrorCode::InvalidExtension),
        4 => Ok(BrokerErrorCode::InvalidFileName),
        5 => Ok(BrokerErrorCode::InvalidFrame),
        6 => Ok(BrokerErrorCode::ProjectTooLarge),
        7 => Ok(BrokerErrorCode::ProtocolMismatch),
        8 => Ok(BrokerErrorCode::ReadFailed),
        9 => Ok(BrokerErrorCode::StaleGrant),
        10 => Ok(BrokerErrorCode::UnknownGrant),
        11 => Ok(BrokerErrorCode::WriteFailed),
        _ => Err(invalid_frame()),
    }
}

fn invalid_frame() -> BrokerError {
    BrokerError::new(
        BrokerErrorCode::InvalidFrame,
        "The native project broker rejected a malformed frame.",
    )
}

fn protocol_io_error() -> BrokerError {
    BrokerError::new(
        BrokerErrorCode::AccessUnavailable,
        "The private native broker channel is unavailable.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    use crate::BROKER_PROTOCOL_VERSION;

    fn round_trip_request(request: BrokerRequest) -> RequestEnvelope {
        let expected = RequestEnvelope {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id: 99,
            request,
        };
        let encoded = encode_request(&expected).unwrap();
        let actual = read_request(&mut encoded.as_slice()).unwrap().unwrap();
        assert_eq!(actual, expected);
        actual
    }

    #[test]
    fn every_request_variant_round_trips() {
        round_trip_request(BrokerRequest::Handshake);
        round_trip_request(BrokerRequest::ListProjects);
        round_trip_request(BrokerRequest::Open {
            name: "cell.vlabproj".into(),
        });
        round_trip_request(BrokerRequest::SaveAs {
            name: "cell.vlabproj".into(),
            bytes: vec![1, 2, 3],
        });
        round_trip_request(BrokerRequest::Save {
            grant_id: GrantId::from_wire(7),
            bytes: vec![4, 5],
        });
        round_trip_request(BrokerRequest::Revoke {
            grant_id: GrantId::from_wire(8),
        });
    }

    #[test]
    fn malformed_headers_payloads_and_trailing_fields_fail_closed() {
        let valid = encode_request(&RequestEnvelope {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id: 1,
            request: BrokerRequest::Handshake,
        })
        .unwrap();
        for length in 1..valid.len() {
            assert!(
                read_request(&mut &valid[..length]).is_err(),
                "length {length}"
            );
        }
        let mut bad_magic = valid.clone();
        bad_magic[0] ^= 0xff;
        assert!(read_request(&mut bad_magic.as_slice()).is_err());
        let mut bad_reserved = valid.clone();
        bad_reserved[11] = 1;
        assert!(read_request(&mut bad_reserved.as_slice()).is_err());
        let mut bad_opcode = valid.clone();
        bad_opcode[10] = 255;
        assert!(read_request(&mut bad_opcode.as_slice()).is_err());
        let mut trailing = valid.clone();
        trailing[20..24].copy_from_slice(&1_u32.to_le_bytes());
        trailing.push(0);
        assert!(read_request(&mut trailing.as_slice()).is_err());
        let mut oversized = valid;
        oversized[20..24]
            .copy_from_slice(&u32::try_from(MAX_FRAME_PAYLOAD + 1).unwrap().to_le_bytes());
        assert!(read_request(&mut oversized.as_slice()).is_err());
    }

    #[test]
    fn response_round_trip_preserves_only_typed_fields() {
        let response = ResponseEnvelope {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id: 8,
            response: Ok(BrokerResponse::Saved {
                display_name: "cell.vlabproj".into(),
                grant_id: GrantId::from_wire(17),
                verified_bytes: 3,
            }),
        };
        let mut bytes = Vec::new();
        write_response(&mut bytes, &response).unwrap();
        assert_eq!(
            decode_response(&mut bytes.as_slice()).unwrap(),
            Some(response)
        );
    }

    #[test]
    fn malformed_utf8_and_declared_length_mismatch_are_rejected() {
        let mut frame = encode_request(&RequestEnvelope {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id: 1,
            request: BrokerRequest::Open {
                name: "x.vlabproj".into(),
            },
        })
        .unwrap();
        frame[HEADER_BYTES + 2] = 0xff;
        assert!(read_request(&mut frame.as_slice()).is_err());

        let mut length = encode_request(&RequestEnvelope {
            protocol_version: BROKER_PROTOCOL_VERSION,
            request_id: 1,
            request: BrokerRequest::Open {
                name: "x.vlabproj".into(),
            },
        })
        .unwrap();
        length[HEADER_BYTES..HEADER_BYTES + 2].copy_from_slice(&200_u16.to_le_bytes());
        assert!(read_request(&mut length.as_slice()).is_err());
    }

    #[test]
    fn clean_eof_is_distinct_from_a_partial_frame() {
        assert_eq!(read_request(&mut io::empty()).unwrap(), None);
        assert!(read_request(&mut &[b'P'][..]).is_err());
    }
}
