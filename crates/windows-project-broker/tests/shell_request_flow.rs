#![cfg(windows)]

use std::ffi::c_void;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use windows_project_broker::protocol::{decode_response, encode_request};
use windows_project_broker::{
    BROKER_PROTOCOL_VERSION, BrokerErrorCode, BrokerRequest, BrokerResponse, RequestEnvelope,
};

fn request(request_id: u64, request: BrokerRequest) -> RequestEnvelope {
    RequestEnvelope {
        protocol_version: BROKER_PROTOCOL_VERSION,
        request_id,
        request,
    }
}

fn send(input: &mut impl Write, request: &RequestEnvelope) {
    input.write_all(&encode_request(request).unwrap()).unwrap();
    input.flush().unwrap();
}

fn unique_project_name() -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "p2-native-broker-test-{}-{nonce}.vlabproj",
        std::process::id()
    )
}

#[test]
fn fixed_helper_typed_request_flow_saves_lists_opens_updates_and_revokes() {
    let project_name = unique_project_name();
    let mut child = Command::new(env!("CARGO_BIN_EXE_windows-project-broker"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = child.stdout.take().unwrap();

    send(&mut input, &request(1, BrokerRequest::Handshake));
    let handshake = decode_response(&mut output).unwrap().unwrap();
    assert_eq!(handshake.request_id, 1);
    if matches!(
        &handshake.response,
        Err(error)
            if constrained_test_identity()
                && error.code == BrokerErrorCode::AttestationFailed
    ) {
        // Codex itself runs with packaged filesystem virtualization. The
        // authoritative Known Folder resolves Scott's profile while writes are
        // redirected into the package LocalCache. That environment must fail
        // closed and is not counted as the positive native product-path run.
        drop(input);
        assert!(child.wait().unwrap().success());
        return;
    }
    let BrokerResponse::Handshake { attestation } = handshake.response.unwrap() else {
        panic!("expected handshake")
    };
    assert!(attestation.validate().is_ok());

    send(
        &mut input,
        &request(
            2,
            BrokerRequest::SaveAs {
                name: project_name.clone(),
                bytes: vec![1, 3, 3, 7],
            },
        ),
    );
    let saved = decode_response(&mut output).unwrap().unwrap();
    let BrokerResponse::Saved {
        grant_id,
        verified_bytes,
        ..
    } = saved.response.unwrap()
    else {
        panic!("expected save")
    };
    assert_eq!(verified_bytes, 4);

    send(&mut input, &request(3, BrokerRequest::ListProjects));
    let listed = decode_response(&mut output).unwrap().unwrap();
    let BrokerResponse::Projects { names } = listed.response.unwrap() else {
        panic!("expected project list")
    };
    assert!(names.contains(&project_name));

    send(
        &mut input,
        &request(
            4,
            BrokerRequest::Open {
                name: project_name.clone(),
            },
        ),
    );
    let opened = decode_response(&mut output).unwrap().unwrap();
    let BrokerResponse::Opened {
        grant_id: open_grant,
        bytes,
        ..
    } = opened.response.unwrap()
    else {
        panic!("expected open")
    };
    assert_eq!(bytes, [1, 3, 3, 7]);

    send(
        &mut input,
        &request(
            5,
            BrokerRequest::Save {
                grant_id: open_grant,
                bytes: vec![4, 5, 6],
            },
        ),
    );
    assert!(matches!(
        decode_response(&mut output).unwrap().unwrap().response,
        Ok(BrokerResponse::Saved {
            verified_bytes: 3,
            ..
        })
    ));

    send(
        &mut input,
        &request(
            6,
            BrokerRequest::Revoke {
                grant_id: open_grant,
            },
        ),
    );
    assert_eq!(
        decode_response(&mut output)
            .unwrap()
            .unwrap()
            .response
            .unwrap(),
        BrokerResponse::Revoked
    );
    send(
        &mut input,
        &request(
            7,
            BrokerRequest::Save {
                grant_id: open_grant,
                bytes: vec![9],
            },
        ),
    );
    assert_eq!(
        decode_response(&mut output)
            .unwrap()
            .unwrap()
            .response
            .unwrap_err()
            .code,
        BrokerErrorCode::UnknownGrant
    );

    send(&mut input, &request(8, BrokerRequest::Revoke { grant_id }));
    let _ = decode_response(&mut output).unwrap().unwrap();
    drop(input);
    assert!(child.wait().unwrap().success());
    let project_path = authoritative_project_root().join(project_name);
    assert_eq!(fs::read(&project_path).unwrap(), [4, 5, 6]);
    fs::remove_file(project_path).unwrap();
}

fn authoritative_project_root() -> PathBuf {
    let mut raw = std::ptr::null_mut();
    assert!(
        unsafe {
            SHGetKnownFolderPath(&FOLDER_ID_LOCAL_APP_DATA, 0, std::ptr::null_mut(), &mut raw)
        } >= 0
    );
    assert!(!raw.is_null());
    let mut length = 0_usize;
    while unsafe { *raw.add(length) } != 0 {
        length += 1;
    }
    let value = String::from_utf16(unsafe { std::slice::from_raw_parts(raw, length) }).unwrap();
    unsafe { CoTaskMemFree(raw.cast()) };
    PathBuf::from(value).join("GovsPLC").join("Projects")
}

#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

const FOLDER_ID_LOCAL_APP_DATA: Guid = Guid {
    data1: 0xf1b3_2785,
    data2: 0x6fba,
    data3: 0x4fcf,
    data4: [0x9d, 0x55, 0x7b, 0x8e, 0x7f, 0x15, 0x70, 0x91],
};

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHGetKnownFolderPath(
        folder_id: *const Guid,
        flags: u32,
        token: *mut c_void,
        path: *mut *mut u16,
    ) -> i32;
}

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoTaskMemFree(memory: *mut c_void);
}

#[test]
fn arbitrary_command_line_fails_before_broker_startup() {
    let output = Command::new(env!("CARGO_BIN_EXE_windows-project-broker"))
        .arg("--path")
        .arg(r"C:\arbitrary\target.vlabproj")
        .output()
        .unwrap();
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn malformed_names_never_escape_the_typed_protocol() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_windows-project-broker"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = child.stdout.take().unwrap();
    send(&mut input, &request(1, BrokerRequest::ListProjects));
    let before_envelope = decode_response(&mut output).unwrap().unwrap();
    if matches!(
        &before_envelope.response,
        Err(error)
            if constrained_test_identity()
                && error.code == BrokerErrorCode::AttestationFailed
    ) {
        // This is the expected packaged-token virtualization denial. The
        // interactive fixed-local shell e2e supplies the positive flow.
        drop(input);
        assert!(child.wait().unwrap().success());
        return;
    }
    let before = before_envelope.response.unwrap();
    for (index, name) in [
        r"C:\escape.vlabproj",
        r"\\server\share\escape.vlabproj",
        r"\\.\pipe\escape.vlabproj",
        "file://server/escape.vlabproj",
        "PRN.vlabproj",
        "project:stream.vlabproj",
    ]
    .into_iter()
    .enumerate()
    {
        let request_id = u64::try_from(index + 2).unwrap();
        send(
            &mut input,
            &request(
                request_id,
                BrokerRequest::SaveAs {
                    name: name.into(),
                    bytes: vec![1],
                },
            ),
        );
        let response = decode_response(&mut output).unwrap().unwrap();
        assert_eq!(response.request_id, request_id);
        assert!(matches!(
            response.response.unwrap_err().code,
            BrokerErrorCode::InvalidFileName | BrokerErrorCode::InvalidExtension
        ));
    }
    send(&mut input, &request(100, BrokerRequest::ListProjects));
    let after = decode_response(&mut output)
        .unwrap()
        .unwrap()
        .response
        .unwrap();
    assert_eq!(after, before);
    drop(input);
    assert!(child.wait().unwrap().success());
}

fn current_process_is_packaged() -> bool {
    let mut length = 0_u32;
    matches!(
        unsafe { GetCurrentPackageFullName(&mut length, std::ptr::null_mut()) },
        0 | 122
    )
}

fn constrained_test_identity() -> bool {
    if current_process_is_packaged() {
        return true;
    }
    let mut buffer = [0_u16; 256];
    let mut length = u32::try_from(buffer.len()).unwrap();
    if unsafe { GetUserNameW(buffer.as_mut_ptr(), &mut length) } == 0 || length < 2 {
        return false;
    }
    let name = String::from_utf16_lossy(&buffer[..usize::try_from(length - 1).unwrap()]);
    name.eq_ignore_ascii_case("CodexSandboxOffline")
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentPackageFullName(
        package_full_name_length: *mut u32,
        package_full_name: *mut u16,
    ) -> i32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn GetUserNameW(buffer: *mut u16, buffer_length: *mut u32) -> i32;
}
