use std::ffi::{OsStr, c_void};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    AttestedFile, BROKER_PROTOCOL_VERSION, BackingAttestation, BrokerError, BrokerErrorCode,
    FixedFileSystem, MAX_PROJECT_BYTES, ProjectFileName, ProjectStorage,
};

const DRIVE_FIXED: u32 = 3;
const FILE_ATTRIBUTE_DEVICE: u32 = 0x0000_0040;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
const FILE_ATTRIBUTE_VIRTUAL: u32 = 0x0001_0000;
const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
const FILE_ATTRIBUTE_PINNED: u32 = 0x0008_0000;
const FILE_ATTRIBUTE_UNPINNED: u32 = 0x0010_0000;
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
const UNSAFE_BACKING_ATTRIBUTES: u32 = FILE_ATTRIBUTE_DEVICE
    | FILE_ATTRIBUTE_REPARSE_POINT
    | FILE_ATTRIBUTE_OFFLINE
    | FILE_ATTRIBUTE_VIRTUAL
    | FILE_ATTRIBUTE_RECALL_ON_OPEN
    | FILE_ATTRIBUTE_PINNED
    | FILE_ATTRIBUTE_UNPINNED
    | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS;

const FILE_SHARE_READ: u32 = 0x0000_0001;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const FILE_SHARE_DELETE: u32 = 0x0000_0004;
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
const FILE_FLAG_WRITE_THROUGH: u32 = 0x8000_0000;
const FILE_REMOTE_PROTOCOL_INFO_CLASS: i32 = 13;
const ERROR_INVALID_FUNCTION: i32 = 1;
const ERROR_NOT_SUPPORTED: i32 = 50;
const ERROR_INVALID_PARAMETER: i32 = 87;
const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;
const REPLACEFILE_WRITE_THROUGH: u32 = 0x0000_0001;
const FILE_NAME_NORMALIZED: u32 = 0;
const VOLUME_NAME_DOS: u32 = 0;
const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002d_1400;
const IOCTL_STORAGE_GET_HOTPLUG_INFO: u32 = 0x002d_0c14;
const STORAGE_DEVICE_PROPERTY: u32 = 0;
const PROPERTY_STANDARD_QUERY: u32 = 0;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsFileToken {
    volume_serial: u64,
    file_index: u64,
    size: u64,
    last_write_time: u64,
    content_sha256: [u8; 32],
}

impl WindowsFileToken {
    fn same_metadata(&self, other: &Self) -> bool {
        self.volume_serial == other.volume_serial
            && self.file_index == other.file_index
            && self.size == other.size
            && self.last_write_time == other.last_write_time
    }
}

pub struct WindowsProjectStorage {
    root: PathBuf,
    volume_serial: u64,
    file_system: FixedFileSystem,
}

impl WindowsProjectStorage {
    pub fn initialize() -> Result<Self, BrokerError> {
        let local_app_data = authoritative_local_app_data()?;
        validate_absolute_drive_path(&local_app_data)?;
        let (volume_serial, file_system) = attest_volume(&local_app_data)?;
        attest_path_chain(
            &local_app_data,
            &local_app_data,
            volume_serial,
            ExpectedKind::Directory,
        )?;

        let application_root = local_app_data.join("GovsPLC");
        create_and_attest_directory(&local_app_data, &application_root, volume_serial)?;
        let project_root = application_root.join("Projects");
        create_and_attest_directory(&local_app_data, &project_root, volume_serial)?;

        Ok(Self {
            root: project_root,
            volume_serial,
            file_system,
        })
    }

    fn reattest_root(&self) -> Result<(), BrokerError> {
        let (serial, file_system) = attest_volume(&self.root)?;
        if serial != self.volume_serial || file_system != self.file_system {
            return Err(attestation_failed());
        }
        attest_path_chain(
            &self.root,
            &self.root,
            self.volume_serial,
            ExpectedKind::Directory,
        )?;
        Ok(())
    }

    fn inspect_metadata(
        &self,
        name: &ProjectFileName,
    ) -> Result<AttestedFile<WindowsFileToken>, BrokerError> {
        self.reattest_root()?;
        let path = self.root.join(name.as_str());
        let metadata = fs::symlink_metadata(&path).map_err(|_| read_failed())?;
        classify_attributes(metadata.file_attributes(), ExpectedKind::RegularFile)?;
        let file = open_for_attributes(&path, ExpectedKind::RegularFile)?;
        let token =
            attest_open_handle(&file, &path, self.volume_serial, ExpectedKind::RegularFile)?;
        let size = usize::try_from(token.size).map_err(|_| read_failed())?;
        if size == 0 || size > MAX_PROJECT_BYTES {
            return Err(BrokerError::new(
                BrokerErrorCode::ProjectTooLarge,
                "The project file is outside the admitted 1-byte to 32-MiB range.",
            ));
        }
        Ok(AttestedFile {
            name: name.clone(),
            token,
            size,
        })
    }

    fn inspect_content(
        &self,
        name: &ProjectFileName,
    ) -> Result<AttestedFile<WindowsFileToken>, BrokerError> {
        let metadata = self.inspect_metadata(name)?;
        let path = self.root.join(name.as_str());
        let mut file = open_for_read(&path)?;
        let (bytes, token) = read_content_bound(&mut file, &path, self.volume_serial)?;
        if !token.same_metadata(&metadata.token) || bytes.len() != metadata.size {
            return Err(BrokerError::new(
                BrokerErrorCode::StaleGrant,
                "The selected project changed between metadata admission and content binding.",
            ));
        }
        Ok(AttestedFile {
            name: name.clone(),
            token,
            size: bytes.len(),
        })
    }

    fn write_verified_temp(
        &self,
        bytes: &[u8],
    ) -> Result<(PathBuf, File, WindowsFileToken), BrokerError> {
        let temp_name = format!(
            ".p2-native-{}.tmp",
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let path = self.root.join(temp_name);
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create_new(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_WRITE_THROUGH);
        let mut file = options.open(&path).map_err(|_| write_failed())?;
        let initial =
            attest_open_handle(&file, &path, self.volume_serial, ExpectedKind::RegularFile)?;
        if initial.size != 0 {
            let _ = fs::remove_file(&path);
            return Err(write_failed());
        }
        let result = (|| {
            file.write_all(bytes).map_err(|_| write_failed())?;
            file.sync_all().map_err(|_| write_failed())?;
            file.seek(SeekFrom::Start(0)).map_err(|_| write_failed())?;
            let mut reopened = Vec::with_capacity(bytes.len());
            Read::by_ref(&mut file)
                .take(u64::try_from(MAX_PROJECT_BYTES + 1).unwrap())
                .read_to_end(&mut reopened)
                .map_err(|_| write_failed())?;
            if reopened != bytes {
                return Err(write_failed());
            }
            let token =
                attest_open_handle(&file, &path, self.volume_serial, ExpectedKind::RegularFile)?;
            if usize::try_from(token.size).ok() != Some(bytes.len()) {
                return Err(write_failed());
            }
            Ok(bind_content(token, &reopened))
        })();
        match result {
            Ok(token) => Ok((path, file, token)),
            Err(error) => {
                drop(file);
                let _ = fs::remove_file(&path);
                Err(error)
            }
        }
    }

    fn commit_new_temp(
        &self,
        temp: &Path,
        temp_handle: &File,
        temp_token: &WindowsFileToken,
        target: &Path,
    ) -> Result<(), BrokerError> {
        self.reattest_root()?;
        let current_temp = attest_open_handle(
            temp_handle,
            temp,
            self.volume_serial,
            ExpectedKind::RegularFile,
        )?;
        if !current_temp.same_metadata(temp_token) {
            return Err(attestation_failed());
        }
        let temp_wide = null_terminated(temp.as_os_str());
        let target_wide = null_terminated(target.as_os_str());
        let succeeded = unsafe {
            MoveFileExW(
                temp_wide.as_ptr(),
                target_wide.as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        };
        if succeeded == 0 {
            return Err(write_failed());
        }
        let committed = attest_open_handle(
            temp_handle,
            target,
            self.volume_serial,
            ExpectedKind::RegularFile,
        )?;
        if !committed.same_metadata(temp_token) {
            return Err(attestation_failed());
        }
        Ok(())
    }

    fn rollback_atomic_replace(
        &self,
        target: &Path,
        backup: &Path,
        desired: &WindowsFileToken,
    ) -> Result<(), BrokerError> {
        let quarantine = self.root.join(format!(
            ".p2-native-rollback-{}.tmp",
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        if fs::symlink_metadata(&quarantine).is_ok() {
            return Err(attestation_failed());
        }
        let target_wide = null_terminated(target.as_os_str());
        let backup_wide = null_terminated(backup.as_os_str());
        let quarantine_wide = null_terminated(quarantine.as_os_str());
        let restored = unsafe {
            ReplaceFileW(
                target_wide.as_ptr(),
                backup_wide.as_ptr(),
                quarantine_wide.as_ptr(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if restored == 0 {
            return Err(attestation_failed());
        }
        let mut restored_handle = open_for_read(target)?;
        let (_, restored_token) =
            read_content_bound(&mut restored_handle, target, self.volume_serial)?;
        drop(restored_handle);
        let _ = fs::remove_file(quarantine);
        if &restored_token != desired {
            return Err(attestation_failed());
        }
        Ok(())
    }

    fn overwrite_attested(
        &mut self,
        name: &ProjectFileName,
        expected: &WindowsFileToken,
        bytes: &[u8],
    ) -> Result<AttestedFile<WindowsFileToken>, BrokerError> {
        self.reattest_root()?;
        let path = self.root.join(name.as_str());
        let mut options = OpenOptions::new();
        options
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
        let mut handle = options.open(&path).map_err(|_| {
            BrokerError::new(
                BrokerErrorCode::StaleGrant,
                "The active project could not be identity-bound for atomic replacement.",
            )
        })?;
        let (_, current) = read_content_bound(&mut handle, &path, self.volume_serial)?;
        if &current != expected || current.size == 0 || current.size > MAX_PROJECT_BYTES as u64 {
            return Err(BrokerError::new(
                BrokerErrorCode::StaleGrant,
                "The active grant no longer identifies the opened project file.",
            ));
        }
        let (temp, temp_handle, temp_token) = self.write_verified_temp(bytes)?;
        let (_, current_before_replace) =
            read_content_bound(&mut handle, &path, self.volume_serial)?;
        let temp_before_replace = attest_open_handle(
            &temp_handle,
            &temp,
            self.volume_serial,
            ExpectedKind::RegularFile,
        )?;
        if current_before_replace != current || !temp_before_replace.same_metadata(&temp_token) {
            drop(temp_handle);
            let _ = fs::remove_file(temp);
            return Err(BrokerError::new(
                BrokerErrorCode::StaleGrant,
                "The target or verified replacement identity changed before commit.",
            ));
        }
        let backup = self.root.join(format!(
            ".p2-native-backup-{}.tmp",
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        if fs::symlink_metadata(&backup).is_ok() {
            drop(temp_handle);
            let _ = fs::remove_file(temp);
            return Err(attestation_failed());
        }
        let target_wide = null_terminated(path.as_os_str());
        let temp_wide = null_terminated(temp.as_os_str());
        let backup_wide = null_terminated(backup.as_os_str());
        let replaced = unsafe {
            ReplaceFileW(
                target_wide.as_ptr(),
                temp_wide.as_ptr(),
                backup_wide.as_ptr(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if replaced == 0 {
            drop(temp_handle);
            let _ = fs::remove_file(temp);
            return Err(write_failed());
        }
        let mut displaced_handle = open_for_read(&backup)?;
        let (_, displaced_token) =
            read_content_bound(&mut displaced_handle, &backup, self.volume_serial)?;
        if displaced_token != current {
            drop(displaced_handle);
            drop(temp_handle);
            self.rollback_atomic_replace(&path, &backup, &displaced_token)?;
            return Err(BrokerError::new(
                BrokerErrorCode::StaleGrant,
                "A different target identity was atomically displaced; it was restored.",
            ));
        }
        let committed_token = attest_open_handle(
            &temp_handle,
            &path,
            self.volume_serial,
            ExpectedKind::RegularFile,
        )
        .map(|token| bind_content(token, bytes));
        let final_file = self.inspect_content(name);
        let valid = matches!(
            (&committed_token, &final_file),
            (Ok(committed), Ok(file))
                if committed == &temp_token
                    && file.token == temp_token
                    && file.size == bytes.len()
        );
        if !valid {
            drop(displaced_handle);
            drop(temp_handle);
            self.rollback_atomic_replace(&path, &backup, &current)?;
            return Err(write_failed());
        }
        drop(displaced_handle);
        drop(temp_handle);
        let _ = fs::remove_file(&backup);
        let final_file = final_file.map_err(|_| write_failed())?;
        let reopened = self
            .read_attested(&final_file)
            .map_err(|_| write_failed())?;
        if reopened != bytes {
            return Err(write_failed());
        }
        Ok(AttestedFile {
            name: name.clone(),
            token: final_file.token,
            size: bytes.len(),
        })
    }
}

fn authoritative_local_app_data() -> Result<PathBuf, BrokerError> {
    let local = authoritative_known_folder(&FOLDER_ID_LOCAL_APP_DATA)?;
    let profile = authoritative_known_folder(&FOLDER_ID_PROFILE)?;
    let expected = profile.join("AppData").join("Local");
    if !same_dos_path(&local, &expected) {
        return Err(attestation_failed());
    }
    Ok(local)
}

fn authoritative_known_folder(folder_id: &Guid) -> Result<PathBuf, BrokerError> {
    let mut raw = std::ptr::null_mut();
    let status = unsafe { SHGetKnownFolderPath(folder_id, 0, std::ptr::null_mut(), &mut raw) };
    if status < 0 || raw.is_null() {
        return Err(access_unavailable());
    }
    let result = (|| {
        let mut length = 0_usize;
        while unsafe { *raw.add(length) } != 0 {
            length = length.checked_add(1).ok_or_else(access_unavailable)?;
            if length > 32_767 {
                return Err(access_unavailable());
            }
        }
        let value = unsafe { std::slice::from_raw_parts(raw, length) };
        let value = String::from_utf16(value).map_err(|_| access_unavailable())?;
        let path = PathBuf::from(value);
        validate_absolute_drive_path(&path)?;
        Ok(path)
    })();
    unsafe { CoTaskMemFree(raw.cast()) };
    result
}

impl ProjectStorage for WindowsProjectStorage {
    type Token = WindowsFileToken;

    fn attest_root(&mut self) -> Result<BackingAttestation, BrokerError> {
        self.reattest_root()?;
        Ok(BackingAttestation {
            protocol_version: BROKER_PROTOCOL_VERSION,
            file_system: self.file_system,
            volume_serial: self.volume_serial,
            fixed_drive: true,
            native_local: true,
            provider_backed: false,
            redirected: false,
            removable: false,
            special: false,
        })
    }

    fn list_projects(&mut self) -> Result<Vec<AttestedFile<Self::Token>>, BrokerError> {
        self.reattest_root()?;
        let mut projects = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|_| read_failed())? {
            let entry = entry.map_err(|_| read_failed())?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| attestation_failed())?;
            if name.starts_with(".p2-native-") && name.ends_with(".tmp") {
                return Err(attestation_failed());
            }
            let name = ProjectFileName::parse(&name).map_err(|_| attestation_failed())?;
            // List is deliberately metadata-only. Project bytes are not read
            // until the host-owned chooser returns a selected base name.
            projects.push(self.inspect_metadata(&name)?);
        }
        Ok(projects)
    }

    fn inspect_existing(
        &mut self,
        name: &ProjectFileName,
    ) -> Result<AttestedFile<Self::Token>, BrokerError> {
        self.inspect_content(name)
    }

    fn read_attested(&mut self, file: &AttestedFile<Self::Token>) -> Result<Vec<u8>, BrokerError> {
        let path = self.root.join(file.name.as_str());
        let mut handle = open_for_read(&path)?;
        let (bytes, token) = read_content_bound(&mut handle, &path, self.volume_serial)?;
        if token != file.token || bytes.len() != file.size {
            return Err(BrokerError::new(
                BrokerErrorCode::StaleGrant,
                "The project changed before selected-byte I/O.",
            ));
        }
        Ok(bytes)
    }

    fn replace_verified(
        &mut self,
        name: &ProjectFileName,
        expected: Option<&Self::Token>,
        bytes: &[u8],
    ) -> Result<AttestedFile<Self::Token>, BrokerError> {
        self.reattest_root()?;
        let target = self.root.join(name.as_str());
        let existing = match fs::symlink_metadata(&target) {
            Ok(_) if expected.is_some() => Some(self.inspect_content(name)?),
            Ok(_) => Some(self.inspect_metadata(name)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(write_failed()),
        };
        if let Some(expected) = expected
            && existing.as_ref().map(|file| &file.token) != Some(expected)
        {
            return Err(BrokerError::new(
                BrokerErrorCode::StaleGrant,
                "The active grant no longer identifies the current project file.",
            ));
        }
        if let Some(expected) = expected {
            return self.overwrite_attested(name, expected, bytes);
        }
        if existing.is_some() {
            return Err(BrokerError::new(
                BrokerErrorCode::WriteFailed,
                "Save As is create-only and will not overwrite an existing project name.",
            ));
        }
        let (temp, temp_handle, temp_token) = self.write_verified_temp(bytes)?;
        if !matches!(
            fs::symlink_metadata(&target),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound
        ) {
            drop(temp_handle);
            let _ = fs::remove_file(&temp);
            return Err(BrokerError::new(
                BrokerErrorCode::WriteFailed,
                "The create-only Save As target appeared while the verified file was prepared.",
            ));
        }
        let replace_result = self.commit_new_temp(&temp, &temp_handle, &temp_token, &target);
        if replace_result.is_err() {
            drop(temp_handle);
            let _ = fs::remove_file(&temp);
            return replace_result.and_then(|()| unreachable!());
        }
        drop(temp_handle);
        let final_file = self.inspect_content(name).map_err(|_| write_failed())?;
        if final_file.size != bytes.len() {
            return Err(write_failed());
        }
        let reopened = self
            .read_attested(&final_file)
            .map_err(|_| write_failed())?;
        if reopened != bytes {
            return Err(write_failed());
        }
        Ok(final_file)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpectedKind {
    Directory,
    RegularFile,
}

fn create_and_attest_directory(
    safe_ancestor: &Path,
    path: &Path,
    volume_serial: u64,
) -> Result<(), BrokerError> {
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or_else(attestation_failed)?;
            attest_path_chain(
                safe_ancestor,
                parent,
                volume_serial,
                ExpectedKind::Directory,
            )?;
            fs::create_dir(path).map_err(|_| access_unavailable())?;
        }
        Err(_) => return Err(attestation_failed()),
    }
    attest_path_chain(safe_ancestor, path, volume_serial, ExpectedKind::Directory)?;
    Ok(())
}

fn attest_volume(path: &Path) -> Result<(u64, FixedFileSystem), BrokerError> {
    let root = drive_root(path)?;
    attest_native_system_volume(&root)?;
    let root_wide = null_terminated(root.as_os_str());
    if unsafe { GetDriveTypeW(root_wide.as_ptr()) } != DRIVE_FIXED {
        return Err(attestation_failed());
    }
    let mut serial = 0_u32;
    let mut file_system = [0_u16; 32];
    let succeeded = unsafe {
        GetVolumeInformationW(
            root_wide.as_ptr(),
            std::ptr::null_mut(),
            0,
            &mut serial,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            file_system.as_mut_ptr(),
            u32::try_from(file_system.len()).unwrap(),
        )
    };
    if succeeded == 0 {
        return Err(attestation_failed());
    }
    let end = file_system
        .iter()
        .position(|value| *value == 0)
        .ok_or_else(attestation_failed)?;
    let name = String::from_utf16(&file_system[..end]).map_err(|_| attestation_failed())?;
    let file_system = match name.to_ascii_uppercase().as_str() {
        "NTFS" => FixedFileSystem::Ntfs,
        "REFS" => FixedFileSystem::Refs,
        _ => return Err(attestation_failed()),
    };
    Ok((u64::from(serial), file_system))
}

fn attest_native_system_volume(root: &Path) -> Result<(), BrokerError> {
    let mut windows_directory = [0_u16; 32_768];
    let length = unsafe {
        GetWindowsDirectoryW(
            windows_directory.as_mut_ptr(),
            u32::try_from(windows_directory.len()).unwrap(),
        )
    };
    if length == 0 || usize::try_from(length).unwrap() >= windows_directory.len() {
        return Err(attestation_failed());
    }
    let windows_path = PathBuf::from(
        String::from_utf16(&windows_directory[..usize::try_from(length).unwrap()])
            .map_err(|_| attestation_failed())?,
    );
    if !same_dos_path(&drive_root(&windows_path)?, root) {
        return Err(attestation_failed());
    }

    let root_text = root.to_str().ok_or_else(attestation_failed)?;
    let volume_path = format!(r"\\.\{}:", &root_text[..1]);
    let mut options = OpenOptions::new();
    options
        .access_mode(0)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
    let volume = options
        .open(volume_path)
        .map_err(|_| attestation_failed())?;
    let handle = volume.as_raw_handle().cast::<c_void>();
    let query = StoragePropertyQuery {
        property_id: STORAGE_DEVICE_PROPERTY,
        query_type: PROPERTY_STANDARD_QUERY,
        additional_parameters: [0],
    };
    let mut descriptor = StorageDeviceDescriptor::default();
    let mut returned = 0_u32;
    if unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            std::ptr::from_ref(&query).cast_mut().cast(),
            u32::try_from(std::mem::size_of_val(&query)).unwrap(),
            std::ptr::from_mut(&mut descriptor).cast(),
            u32::try_from(std::mem::size_of_val(&descriptor)).unwrap(),
            &mut returned,
            std::ptr::null_mut(),
        )
    } == 0
        || usize::try_from(returned).unwrap() < std::mem::size_of::<StorageDeviceDescriptor>()
    {
        return Err(attestation_failed());
    }
    let mut hotplug = StorageHotplugInfo::default();
    returned = 0;
    if unsafe {
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_GET_HOTPLUG_INFO,
            std::ptr::null_mut(),
            0,
            std::ptr::from_mut(&mut hotplug).cast(),
            u32::try_from(std::mem::size_of_val(&hotplug)).unwrap(),
            &mut returned,
            std::ptr::null_mut(),
        )
    } == 0
        || usize::try_from(returned).unwrap() < std::mem::size_of::<StorageHotplugInfo>()
    {
        return Err(attestation_failed());
    }
    classify_storage_device(
        descriptor.bus_type,
        descriptor.removable_media != 0,
        hotplug.media_removable != 0,
        hotplug.media_hotplug != 0,
        hotplug.device_hotplug != 0,
    )
}

fn classify_storage_device(
    bus_type: u32,
    removable_media: bool,
    media_removable: bool,
    media_hotplug: bool,
    device_hotplug: bool,
) -> Result<(), BrokerError> {
    // Admit only native fixed-system storage buses. USB, 1394, iSCSI, SD/MMC,
    // virtual/file-backed, Storage Spaces, Fibre Channel, and unknown buses
    // fail closed even when Windows reports DRIVE_FIXED.
    const ADMITTED_NATIVE_BUSES: [u32; 7] = [1, 2, 3, 8, 10, 11, 17];
    if !ADMITTED_NATIVE_BUSES.contains(&bus_type)
        || removable_media
        || media_removable
        || media_hotplug
        || device_hotplug
    {
        return Err(attestation_failed());
    }
    Ok(())
}

fn attest_path_chain(
    safe_ancestor: &Path,
    target: &Path,
    volume_serial: u64,
    final_kind: ExpectedKind,
) -> Result<(), BrokerError> {
    validate_absolute_drive_path(safe_ancestor)?;
    validate_absolute_drive_path(target)?;
    if !path_is_within(target, safe_ancestor) {
        return Err(attestation_failed());
    }
    let root = drive_root(target)?;
    let relative = target
        .strip_prefix(&root)
        .map_err(|_| attestation_failed())?;
    let mut current = root;
    let component_count = relative.components().count();
    for (index, component) in relative.components().enumerate() {
        let std::path::Component::Normal(component) = component else {
            return Err(attestation_failed());
        };
        current.push(component);
        let expected = if index + 1 == component_count {
            final_kind
        } else {
            ExpectedKind::Directory
        };
        let metadata = fs::symlink_metadata(&current).map_err(|_| attestation_failed())?;
        classify_attributes(metadata.file_attributes(), expected)?;
        // The authoritative known-folder target and every descendant are opened
        // and identity-bound. Some constrained Windows tokens cannot open an
        // ancestor profile directory even with zero desired access. Those
        // ancestors are still lstat-checked above, while the final-handle DOS
        // path comparison detects an ancestor redirect before authority is
        // admitted.
        if path_is_within(&current, safe_ancestor) {
            let handle = open_for_attributes(&current, expected)?;
            attest_open_handle(&handle, &current, volume_serial, expected)?;
        }
    }
    Ok(())
}

fn open_for_attributes(path: &Path, kind: ExpectedKind) -> Result<File, BrokerError> {
    let mut options = OpenOptions::new();
    options
        .access_mode(0)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(
            FILE_FLAG_OPEN_REPARSE_POINT
                | if kind == ExpectedKind::Directory {
                    FILE_FLAG_BACKUP_SEMANTICS
                } else {
                    0
                },
        );
    options.open(path).map_err(|_| attestation_failed())
}

fn open_for_read(path: &Path) -> Result<File, BrokerError> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(path).map_err(|_| read_failed())
}

fn bind_content(mut token: WindowsFileToken, bytes: &[u8]) -> WindowsFileToken {
    token.content_sha256 = crate::sha256::sha256(bytes);
    token
}

fn read_content_bound(
    file: &mut File,
    expected_path: &Path,
    expected_volume_serial: u64,
) -> Result<(Vec<u8>, WindowsFileToken), BrokerError> {
    let before = attest_open_handle(
        file,
        expected_path,
        expected_volume_serial,
        ExpectedKind::RegularFile,
    )?;
    if before.size == 0 || before.size > MAX_PROJECT_BYTES as u64 {
        return Err(BrokerError::new(
            BrokerErrorCode::ProjectTooLarge,
            "The project file is outside the admitted 1-byte to 32-MiB range.",
        ));
    }
    file.seek(SeekFrom::Start(0)).map_err(|_| read_failed())?;
    let mut bytes = Vec::with_capacity(usize::try_from(before.size).unwrap());
    Read::by_ref(file)
        .take(u64::try_from(MAX_PROJECT_BYTES + 1).unwrap())
        .read_to_end(&mut bytes)
        .map_err(|_| read_failed())?;
    let after = attest_open_handle(
        file,
        expected_path,
        expected_volume_serial,
        ExpectedKind::RegularFile,
    )?;
    if before != after || bytes.len() != usize::try_from(after.size).unwrap() {
        return Err(BrokerError::new(
            BrokerErrorCode::StaleGrant,
            "The project identity or provenance changed while its content was bound.",
        ));
    }
    let token = bind_content(after, &bytes);
    Ok((bytes, token))
}

fn attest_open_handle(
    file: &File,
    expected_path: &Path,
    expected_volume_serial: u64,
    expected_kind: ExpectedKind,
) -> Result<WindowsFileToken, BrokerError> {
    let handle = file.as_raw_handle().cast::<c_void>();
    let mut information = ByHandleFileInformation::default();
    if unsafe { GetFileInformationByHandle(handle, &mut information) } == 0 {
        return Err(attestation_failed());
    }
    classify_attributes(information.file_attributes, expected_kind)?;
    classify_link_count(information.number_of_links, expected_kind)?;
    if u64::from(information.volume_serial_number) != expected_volume_serial {
        return Err(attestation_failed());
    }
    ensure_not_remote(handle)?;
    let final_path = final_dos_path(handle)?;
    if !same_dos_path(&final_path, expected_path) {
        return Err(attestation_failed());
    }
    Ok(WindowsFileToken {
        volume_serial: u64::from(information.volume_serial_number),
        file_index: (u64::from(information.file_index_high) << 32)
            | u64::from(information.file_index_low),
        size: (u64::from(information.file_size_high) << 32) | u64::from(information.file_size_low),
        last_write_time: (u64::from(information.last_write_time.high_date_time) << 32)
            | u64::from(information.last_write_time.low_date_time),
        content_sha256: [0; 32],
    })
}

fn classify_link_count(
    number_of_links: u32,
    expected_kind: ExpectedKind,
) -> Result<(), BrokerError> {
    if expected_kind == ExpectedKind::RegularFile && number_of_links != 1 {
        return Err(attestation_failed());
    }
    Ok(())
}

fn ensure_not_remote(handle: *mut c_void) -> Result<(), BrokerError> {
    let mut remote = [0_u64; 15];
    let succeeded = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FILE_REMOTE_PROTOCOL_INFO_CLASS,
            remote.as_mut_ptr().cast::<c_void>(),
            u32::try_from(std::mem::size_of_val(&remote)).unwrap(),
        )
    };
    if succeeded != 0 {
        return Err(attestation_failed());
    }
    let error = std::io::Error::last_os_error()
        .raw_os_error()
        .unwrap_or_default();
    if !matches!(
        error,
        ERROR_INVALID_FUNCTION | ERROR_NOT_SUPPORTED | ERROR_INVALID_PARAMETER
    ) {
        return Err(attestation_failed());
    }
    Ok(())
}

fn final_dos_path(handle: *mut c_void) -> Result<PathBuf, BrokerError> {
    let flags = FILE_NAME_NORMALIZED | VOLUME_NAME_DOS;
    let required = unsafe { GetFinalPathNameByHandleW(handle, std::ptr::null_mut(), 0, flags) };
    if required == 0 || required > 32_767 {
        return Err(attestation_failed());
    }
    let mut buffer = vec![0_u16; usize::try_from(required).unwrap() + 1];
    let written = unsafe {
        GetFinalPathNameByHandleW(
            handle,
            buffer.as_mut_ptr(),
            u32::try_from(buffer.len()).unwrap(),
            flags,
        )
    };
    if written == 0 || usize::try_from(written).unwrap() >= buffer.len() {
        return Err(attestation_failed());
    }
    let value = String::from_utf16(&buffer[..usize::try_from(written).unwrap()])
        .map_err(|_| attestation_failed())?;
    let value = value.strip_prefix(r"\\?\").unwrap_or(&value);
    let path = PathBuf::from(value);
    validate_absolute_drive_path(&path)?;
    Ok(path)
}

fn classify_attributes(attributes: u32, expected: ExpectedKind) -> Result<(), BrokerError> {
    if attributes & UNSAFE_BACKING_ATTRIBUTES != 0 {
        return Err(attestation_failed());
    }
    let is_directory = attributes & FILE_ATTRIBUTE_DIRECTORY != 0;
    if is_directory != (expected == ExpectedKind::Directory) {
        return Err(attestation_failed());
    }
    Ok(())
}

fn validate_absolute_drive_path(path: &Path) -> Result<(), BrokerError> {
    let value = path.to_str().ok_or_else(attestation_failed)?;
    let bytes = value.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || !matches!(bytes[2], b'\\' | b'/')
        || value.starts_with(r"\\")
        || value.starts_with(r"\\?")
        || value.starts_with(r"\\.")
        || value[2..].contains(':')
    {
        return Err(attestation_failed());
    }
    for component in value[3..].split(['\\', '/']) {
        if component.is_empty() {
            continue;
        }
        if component == "."
            || component == ".."
            || component.ends_with(['.', ' '])
            || component.chars().any(|character| character.is_control())
        {
            return Err(attestation_failed());
        }
    }
    Ok(())
}

fn drive_root(path: &Path) -> Result<PathBuf, BrokerError> {
    validate_absolute_drive_path(path)?;
    let value = path.to_str().ok_or_else(attestation_failed)?;
    Ok(PathBuf::from(&value[..3]))
}

fn path_is_within(path: &Path, ancestor: &Path) -> bool {
    let Some(path) = path.to_str() else {
        return false;
    };
    let Some(ancestor) = ancestor.to_str() else {
        return false;
    };
    let path = normalize_dos_path(path);
    let ancestor = normalize_dos_path(ancestor);
    path == ancestor
        || path
            .strip_prefix(&ancestor)
            .is_some_and(|suffix| suffix.starts_with('\\'))
}

fn same_dos_path(left: &Path, right: &Path) -> bool {
    match (left.to_str(), right.to_str()) {
        (Some(left), Some(right)) => normalize_dos_path(left) == normalize_dos_path(right),
        _ => false,
    }
}

fn normalize_dos_path(value: &str) -> String {
    value
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn null_terminated(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn access_unavailable() -> BrokerError {
    BrokerError::new(
        BrokerErrorCode::AccessUnavailable,
        "The fixed Windows project root is unavailable.",
    )
}

fn attestation_failed() -> BrokerError {
    BrokerError::new(
        BrokerErrorCode::AttestationFailed,
        "The target is not proven fixed, native, local, regular, and non-provider-backed.",
    )
}

fn read_failed() -> BrokerError {
    BrokerError::new(
        BrokerErrorCode::ReadFailed,
        "The attested project could not be read.",
    )
}

fn write_failed() -> BrokerError {
    BrokerError::new(
        BrokerErrorCode::WriteFailed,
        "The project replacement did not complete and verify atomically.",
    )
}

#[repr(C)]
#[derive(Default)]
struct FileTime {
    low_date_time: u32,
    high_date_time: u32,
}

#[repr(C)]
#[derive(Default)]
struct ByHandleFileInformation {
    file_attributes: u32,
    creation_time: FileTime,
    last_access_time: FileTime,
    last_write_time: FileTime,
    volume_serial_number: u32,
    file_size_high: u32,
    file_size_low: u32,
    number_of_links: u32,
    file_index_high: u32,
    file_index_low: u32,
}

#[repr(C)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct StoragePropertyQuery {
    property_id: u32,
    query_type: u32,
    additional_parameters: [u8; 1],
}

#[repr(C)]
#[derive(Default)]
struct StorageDeviceDescriptor {
    version: u32,
    size: u32,
    device_type: u8,
    device_type_modifier: u8,
    removable_media: u8,
    command_queueing: u8,
    vendor_id_offset: u32,
    product_id_offset: u32,
    product_revision_offset: u32,
    serial_number_offset: u32,
    bus_type: u32,
    raw_properties_length: u32,
}

#[repr(C)]
#[derive(Default)]
struct StorageHotplugInfo {
    size: u32,
    media_removable: u8,
    media_hotplug: u8,
    device_hotplug: u8,
    write_cache_enable_override: u8,
}

const FOLDER_ID_LOCAL_APP_DATA: Guid = Guid {
    data1: 0xf1b3_2785,
    data2: 0x6fba,
    data3: 0x4fcf,
    data4: [0x9d, 0x55, 0x7b, 0x8e, 0x7f, 0x15, 0x70, 0x91],
};

const FOLDER_ID_PROFILE: Guid = Guid {
    data1: 0x5e6c_858f,
    data2: 0x0e22,
    data3: 0x4760,
    data4: [0x9a, 0xfe, 0xea, 0x33, 0x17, 0xb6, 0x71, 0x73],
};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetDriveTypeW(root_path_name: *const u16) -> u32;
    fn GetWindowsDirectoryW(buffer: *mut u16, size: u32) -> u32;
    fn GetVolumeInformationW(
        root_path_name: *const u16,
        volume_name_buffer: *mut u16,
        volume_name_size: u32,
        volume_serial_number: *mut u32,
        maximum_component_length: *mut u32,
        file_system_flags: *mut u32,
        file_system_name_buffer: *mut u16,
        file_system_name_size: u32,
    ) -> i32;
    fn GetFileInformationByHandle(
        file: *mut c_void,
        information: *mut ByHandleFileInformation,
    ) -> i32;
    fn GetFileInformationByHandleEx(
        file: *mut c_void,
        information_class: i32,
        information: *mut c_void,
        buffer_size: u32,
    ) -> i32;
    fn GetFinalPathNameByHandleW(
        file: *mut c_void,
        path: *mut u16,
        path_size: u32,
        flags: u32,
    ) -> u32;
    fn MoveFileExW(existing_file_name: *const u16, new_file_name: *const u16, flags: u32) -> i32;
    fn ReplaceFileW(
        replaced_file_name: *const u16,
        replacement_file_name: *const u16,
        backup_file_name: *const u16,
        replace_flags: u32,
        exclude: *mut c_void,
        reserved: *mut c_void,
    ) -> i32;
    fn DeviceIoControl(
        device: *mut c_void,
        control_code: u32,
        input: *mut c_void,
        input_size: u32,
        output: *mut c_void,
        output_size: u32,
        bytes_returned: *mut u32,
        overlapped: *mut c_void,
    ) -> i32;
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_provider_remote_redirect_and_special_attribute_fails_closed() {
        for bit in [
            FILE_ATTRIBUTE_DEVICE,
            FILE_ATTRIBUTE_REPARSE_POINT,
            FILE_ATTRIBUTE_OFFLINE,
            FILE_ATTRIBUTE_VIRTUAL,
            FILE_ATTRIBUTE_RECALL_ON_OPEN,
            FILE_ATTRIBUTE_PINNED,
            FILE_ATTRIBUTE_UNPINNED,
            FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
        ] {
            assert!(classify_attributes(bit, ExpectedKind::RegularFile).is_err());
            assert!(
                classify_attributes(bit | FILE_ATTRIBUTE_DIRECTORY, ExpectedKind::Directory)
                    .is_err()
            );
        }
    }

    #[test]
    fn regular_file_and_directory_types_are_not_interchangeable() {
        assert!(classify_attributes(0, ExpectedKind::RegularFile).is_ok());
        assert!(classify_attributes(FILE_ATTRIBUTE_DIRECTORY, ExpectedKind::Directory).is_ok());
        assert!(classify_attributes(0, ExpectedKind::Directory).is_err());
        assert!(classify_attributes(FILE_ATTRIBUTE_DIRECTORY, ExpectedKind::RegularFile).is_err());
    }

    #[test]
    fn hard_link_aliases_and_non_native_or_hotplug_storage_fail_closed() {
        assert!(classify_link_count(1, ExpectedKind::RegularFile).is_ok());
        assert!(classify_link_count(2, ExpectedKind::RegularFile).is_err());
        assert!(classify_link_count(0, ExpectedKind::RegularFile).is_err());

        for rejected_bus in [0, 4, 6, 7, 9, 12, 13, 14, 15, 16, 18, 19, 20] {
            assert!(classify_storage_device(rejected_bus, false, false, false, false).is_err());
        }
        assert!(classify_storage_device(17, false, false, false, false).is_ok());
        assert!(classify_storage_device(17, true, false, false, false).is_err());
        assert!(classify_storage_device(17, false, true, false, false).is_err());
        assert!(classify_storage_device(17, false, false, true, false).is_err());
        assert!(classify_storage_device(17, false, false, false, true).is_err());
    }

    #[test]
    fn absolute_path_shape_rejects_unc_device_verbatim_ads_and_traversal() {
        for path in [
            r"\\server\share\Projects",
            r"\\.\C:\Projects",
            r"\\?\C:\Projects",
            r"C:\Projects\file:stream",
            r"C:\Projects\..\Elsewhere",
            r"C:\Projects\.\file",
            r"C:\Projects\trailing.\file",
            r"C:\Projects\trailing \file",
            r"relative\Projects",
        ] {
            assert!(
                validate_absolute_drive_path(Path::new(path)).is_err(),
                "{path}"
            );
        }
        assert!(validate_absolute_drive_path(Path::new(r"C:\Users\Student\Projects")).is_ok());
    }

    #[test]
    fn root_containment_uses_component_boundary_not_string_prefix() {
        let root = Path::new(r"C:\Users\Student\Projects");
        assert!(path_is_within(
            Path::new(r"c:\users\student\projects\cell.vlabproj"),
            root
        ));
        assert!(!path_is_within(
            Path::new(r"C:\Users\Student\Projects-escape\cell.vlabproj"),
            root
        ));
        assert!(!path_is_within(
            Path::new(r"D:\Projects\cell.vlabproj"),
            root
        ));
    }
}
