#![forbid(unsafe_code)]

use std::env;
use std::io::{self, BufReader, BufWriter};

use windows_project_broker::protocol::{read_request, write_response};
use windows_project_broker::{BROKER_PROTOCOL_VERSION, ProjectFileBroker, ResponseEnvelope};

#[cfg(windows)]
use windows_project_broker::WindowsProjectStorage;

#[cfg(not(windows))]
use windows_project_broker::{BrokerError, BrokerErrorCode};

fn main() {
    run();
}

fn run() {
    if env::args_os().count() != 1 {
        return;
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = BufReader::new(stdin.lock());
    let mut output = BufWriter::new(stdout.lock());

    #[cfg(windows)]
    let initialized = WindowsProjectStorage::initialize().and_then(ProjectFileBroker::initialize);

    #[cfg(not(windows))]
    let initialized: Result<ProjectFileBroker<UnsupportedStorage>, BrokerError> =
        Err(BrokerError::new(
            BrokerErrorCode::AccessUnavailable,
            "The native project-file broker is supported only on Windows.",
        ));

    let mut broker = match initialized {
        Ok(broker) => broker,
        Err(error) => {
            let request = match read_request(&mut input) {
                Ok(Some(request)) => request,
                Ok(None) | Err(_) => return,
            };
            let response = ResponseEnvelope {
                protocol_version: BROKER_PROTOCOL_VERSION,
                request_id: request.request_id,
                response: Err(error),
            };
            let _ = write_response(&mut output, &response);
            return;
        }
    };

    loop {
        let request = match read_request(&mut input) {
            Ok(Some(request)) => request,
            Ok(None) => return,
            Err(_) => return,
        };
        let response = broker.execute(request);
        if write_response(&mut output, &response).is_err() {
            return;
        }
    }
}

#[cfg(not(windows))]
struct UnsupportedStorage;

#[cfg(not(windows))]
impl windows_project_broker::ProjectStorage for UnsupportedStorage {
    type Token = ();

    fn attest_root(&mut self) -> Result<windows_project_broker::BackingAttestation, BrokerError> {
        Err(BrokerError::new(
            BrokerErrorCode::AccessUnavailable,
            "The native project-file broker is supported only on Windows.",
        ))
    }

    fn list_projects(
        &mut self,
    ) -> Result<Vec<windows_project_broker::AttestedFile<Self::Token>>, BrokerError> {
        unreachable!()
    }

    fn inspect_existing(
        &mut self,
        _name: &windows_project_broker::ProjectFileName,
    ) -> Result<windows_project_broker::AttestedFile<Self::Token>, BrokerError> {
        unreachable!()
    }

    fn read_attested(
        &mut self,
        _file: &windows_project_broker::AttestedFile<Self::Token>,
    ) -> Result<Vec<u8>, BrokerError> {
        unreachable!()
    }

    fn replace_verified(
        &mut self,
        _name: &windows_project_broker::ProjectFileName,
        _expected: Option<&Self::Token>,
        _bytes: &[u8],
    ) -> Result<windows_project_broker::AttestedFile<Self::Token>, BrokerError> {
        unreachable!()
    }
}
