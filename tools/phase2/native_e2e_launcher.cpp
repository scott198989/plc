#ifndef _WIN32_WINNT
#define _WIN32_WINNT 0x0A00
#endif

#include <winsock2.h>
#include <windows.h>
#include <ws2tcpip.h>
#include <bcrypt.h>
#include <iphlpapi.h>
#include <shlobj.h>
#include <shellapi.h>
#include <tlhelp32.h>
#include <winioctl.h>

#include <algorithm>
#include <array>
#include <chrono>
#include <cctype>
#include <cstddef>
#include <cstdint>
#include <cwctype>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <limits>
#include <map>
#include <optional>
#include <ranges>
#include <set>
#include <span>
#include <sstream>
#include <stdexcept>
#include <string>
#include <string_view>
#include <thread>
#include <tuple>
#include <vector>

namespace {

constexpr DWORD kUnsafeBackingAttributes =
    FILE_ATTRIBUTE_DEVICE | FILE_ATTRIBUTE_REPARSE_POINT |
    FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_VIRTUAL |
    FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_PINNED |
    FILE_ATTRIBUTE_UNPINNED | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS;
constexpr DWORD kRunTimeoutMilliseconds = 120'000;

struct CandidateFile final {
  std::wstring_view relative_path;
  std::uint64_t bytes;
  std::wstring_view sha256;
};

#include "native_e2e_candidate.h"

[[noreturn]] void fail(const char* message) { throw std::runtime_error(message); }

class HeldHandle final {
 public:
  HeldHandle() = default;
  explicit HeldHandle(HANDLE value) : value_(value) {}
  HeldHandle(const HeldHandle&) = delete;
  HeldHandle& operator=(const HeldHandle&) = delete;
  HeldHandle(HeldHandle&& other) noexcept : value_(other.release()) {}
  HeldHandle& operator=(HeldHandle&& other) noexcept {
    if (this != &other) reset(other.release());
    return *this;
  }
  ~HeldHandle() { reset(); }

  [[nodiscard]] HANDLE get() const noexcept { return value_; }
  [[nodiscard]] bool valid() const noexcept {
    return value_ != nullptr && value_ != INVALID_HANDLE_VALUE;
  }
  [[nodiscard]] HANDLE release() noexcept {
    const auto value = value_;
    value_ = INVALID_HANDLE_VALUE;
    return value;
  }
  void reset(HANDLE next = INVALID_HANDLE_VALUE) noexcept {
    if (valid()) CloseHandle(value_);
    value_ = next;
  }

 private:
  HANDLE value_{INVALID_HANDLE_VALUE};
};

class HeldFind final {
 public:
  explicit HeldFind(HANDLE value) : value_(value) {}
  HeldFind(const HeldFind&) = delete;
  HeldFind& operator=(const HeldFind&) = delete;
  ~HeldFind() {
    if (valid()) FindClose(value_);
  }
  [[nodiscard]] HANDLE get() const noexcept { return value_; }
  [[nodiscard]] bool valid() const noexcept {
    return value_ != nullptr && value_ != INVALID_HANDLE_VALUE;
  }
  void reset() noexcept {
    if (valid()) FindClose(value_);
    value_ = INVALID_HANDLE_VALUE;
  }

 private:
  HANDLE value_{INVALID_HANDLE_VALUE};
};

std::wstring normalized_path(std::wstring value) {
  if (value.starts_with(LR"(\\?\)")) value.erase(0, 4);
  std::ranges::replace(value, L'/', L'\\');
  while (value.size() > 3 && value.back() == L'\\') value.pop_back();
  std::ranges::transform(value, value.begin(), [](wchar_t character) {
    return static_cast<wchar_t>(std::towlower(character));
  });
  return value;
}

std::filesystem::path executable_path() {
  std::wstring buffer(32'768, L'\0');
  const auto length = GetModuleFileNameW(
      nullptr, buffer.data(), static_cast<DWORD>(buffer.size()));
  if (length == 0 || length >= buffer.size()) {
    fail("The native launcher executable identity is unavailable.");
  }
  buffer.resize(length);
  return std::filesystem::path(buffer);
}

std::wstring final_path(HANDLE handle) {
  const auto required = GetFinalPathNameByHandleW(
      handle, nullptr, 0, FILE_NAME_NORMALIZED | VOLUME_NAME_DOS);
  if (required == 0 || required > 32'767) {
    fail("An attested path returned no bounded final DOS identity.");
  }
  std::wstring buffer(static_cast<std::size_t>(required) + 1U, L'\0');
  const auto written = GetFinalPathNameByHandleW(
      handle,
      buffer.data(),
      static_cast<DWORD>(buffer.size()),
      FILE_NAME_NORMALIZED | VOLUME_NAME_DOS);
  if (written == 0 || written >= buffer.size()) {
    fail("An attested path final identity failed closed.");
  }
  buffer.resize(written);
  return normalized_path(std::move(buffer));
}

std::filesystem::path authoritative_local_app_data() {
  PWSTR raw = nullptr;
  if (FAILED(SHGetKnownFolderPath(FOLDERID_LocalAppData, 0, nullptr, &raw)) ||
      raw == nullptr) {
    fail("The authoritative LocalAppData known folder is unavailable.");
  }
  const std::filesystem::path value(raw);
  CoTaskMemFree(raw);
  if (!value.is_absolute() || value.root_path().wstring().size() != 3) {
    fail("The authoritative LocalAppData path is not a bounded drive path.");
  }
  PWSTR profile_raw = nullptr;
  if (FAILED(SHGetKnownFolderPath(FOLDERID_Profile, 0, nullptr, &profile_raw)) ||
      profile_raw == nullptr) {
    fail("The authoritative user profile known folder is unavailable.");
  }
  const std::filesystem::path profile(profile_raw);
  CoTaskMemFree(profile_raw);
  if (!profile.is_absolute() || profile.root_path().wstring().size() != 3 ||
      normalized_path(value.wstring()) !=
          normalized_path((profile / L"AppData" / L"Local").wstring())) {
    fail("LocalAppData is redirected outside the authoritative local user profile.");
  }
  return value;
}

struct VolumeAttestation final {
  std::uint32_t serial{};
  std::wstring file_system;
  std::wstring storage_bus;
};

VolumeAttestation attest_native_system_volume(const std::filesystem::path& path) {
  const auto root = path.root_path().wstring();
  if (root.size() != 3 || GetDriveTypeW(root.c_str()) != DRIVE_FIXED) {
    fail("LocalAppData is not on a fixed drive.");
  }
  std::array<wchar_t, 32'768> windows_directory{};
  const auto windows_length = GetWindowsDirectoryW(
      windows_directory.data(), static_cast<UINT>(windows_directory.size()));
  if (windows_length == 0 || windows_length >= windows_directory.size()) {
    fail("The Windows native system volume could not be resolved.");
  }
  const std::filesystem::path windows_path(
      std::wstring_view(windows_directory.data(), windows_length));
  if (normalized_path(windows_path.root_path().wstring()) !=
      normalized_path(root)) {
    fail("LocalAppData is outside the native Windows system volume.");
  }

  const std::wstring volume_path = LR"(\\.\)" + root.substr(0, 2);
  HeldHandle volume(CreateFileW(
      volume_path.c_str(),
      0,
      FILE_SHARE_READ | FILE_SHARE_WRITE,
      nullptr,
      OPEN_EXISTING,
      0,
      nullptr));
  if (!volume.valid()) fail("The native storage device could not be opened.");

  STORAGE_PROPERTY_QUERY query{};
  query.PropertyId = StorageDeviceProperty;
  query.QueryType = PropertyStandardQuery;
  alignas(STORAGE_DEVICE_DESCRIPTOR) std::array<std::uint8_t, 1'024> descriptor_bytes{};
  DWORD returned = 0;
  if (DeviceIoControl(
          volume.get(),
          IOCTL_STORAGE_QUERY_PROPERTY,
          &query,
          sizeof(query),
          descriptor_bytes.data(),
          static_cast<DWORD>(descriptor_bytes.size()),
          &returned,
          nullptr) == 0 ||
      returned < offsetof(STORAGE_DEVICE_DESCRIPTOR, RawDeviceProperties)) {
    fail("The native storage device descriptor failed closed.");
  }
  const auto& descriptor = *reinterpret_cast<const STORAGE_DEVICE_DESCRIPTOR*>(
      descriptor_bytes.data());
  STORAGE_HOTPLUG_INFO hotplug{};
  returned = 0;
  if (DeviceIoControl(
          volume.get(),
          IOCTL_STORAGE_GET_HOTPLUG_INFO,
          nullptr,
          0,
          &hotplug,
          sizeof(hotplug),
          &returned,
          nullptr) == 0 ||
      returned < sizeof(hotplug)) {
    fail("The native storage hotplug status failed closed.");
  }

  std::wstring bus;
  switch (descriptor.BusType) {
    case BusTypeScsi: bus = L"SCSI"; break;
    case BusTypeAtapi: bus = L"ATAPI"; break;
    case BusTypeAta: bus = L"ATA"; break;
    case BusTypeRAID: bus = L"RAID"; break;
    case BusTypeSas: bus = L"SAS"; break;
    case BusTypeSata: bus = L"SATA"; break;
    case BusTypeNvme: bus = L"NVMe"; break;
    default: fail("The native storage bus is remote, removable, virtual, or unapproved.");
  }
  if (descriptor.RemovableMedia != FALSE || hotplug.MediaRemovable != FALSE ||
      hotplug.MediaHotplug != FALSE || hotplug.DeviceHotplug != FALSE) {
    fail("The native storage device is removable or hotplug-backed.");
  }

  VolumeAttestation result{};
  std::array<wchar_t, 32> file_system{};
  if (GetVolumeInformationW(
          root.c_str(),
          nullptr,
          0,
          reinterpret_cast<LPDWORD>(&result.serial),
          nullptr,
          nullptr,
          file_system.data(),
          static_cast<DWORD>(file_system.size())) == 0) {
    fail("The native storage filesystem could not be attested.");
  }
  result.file_system = file_system.data();
  if (result.file_system != L"NTFS" && result.file_system != L"ReFS") {
    fail("The native storage filesystem is not NTFS or ReFS.");
  }
  result.storage_bus = std::move(bus);
  return result;
}

enum class HardlinkPolicy { require_single, allow_multiple };

HeldHandle open_attested_path(
    const std::filesystem::path& path,
    bool directory,
    std::uint32_t expected_serial,
    DWORD access = 0,
    HardlinkPolicy hardlink_policy = HardlinkPolicy::require_single) {
  const auto desired_access =
      access != 0 ? access : (directory ? 0U : GENERIC_READ);
  HeldHandle authority(CreateFileW(
      path.c_str(),
      desired_access,
      FILE_SHARE_READ,
      nullptr,
      OPEN_EXISTING,
      FILE_FLAG_OPEN_REPARSE_POINT |
          (directory ? FILE_FLAG_BACKUP_SEMANTICS : FILE_FLAG_SEQUENTIAL_SCAN),
      nullptr));
  if (!authority.valid()) fail("A fixed-local authority path could not be opened.");
  BY_HANDLE_FILE_INFORMATION information{};
  if (GetFileInformationByHandle(authority.get(), &information) == 0) {
    fail("A fixed-local authority path has no handle identity.");
  }
  if (information.dwVolumeSerialNumber != expected_serial) {
    fail("A fixed-local authority path has a different volume identity.");
  }
  if ((information.dwFileAttributes & kUnsafeBackingAttributes) != 0) {
    fail("A fixed-local authority path has unsafe backing attributes.");
  }
  if (((information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0) != directory) {
    fail("A fixed-local authority path has the wrong object type.");
  }
  if (!directory && information.nNumberOfLinks == 0) {
    fail("A fixed-local authority file has no hardlink identity.");
  }
  if (!directory && hardlink_policy == HardlinkPolicy::require_single &&
      information.nNumberOfLinks != 1) {
    fail("A fixed-local authority file is not single-link.");
  }
  const auto observed_identity = final_path(authority.get());
  const auto expected_identity = normalized_path(path.wstring());
  if (observed_identity != expected_identity) {
    throw std::runtime_error(
        "A fixed-local authority path canonical identity changed: expected=" +
        std::filesystem::path(expected_identity).string() + " observed=" +
        std::filesystem::path(observed_identity).string());
  }
  FILE_REMOTE_PROTOCOL_INFO remote{};
  SetLastError(ERROR_SUCCESS);
  if (GetFileInformationByHandleEx(
          authority.get(), FileRemoteProtocolInfo, &remote, sizeof(remote)) != 0) {
    fail("A fixed-local authority path is remote.");
  }
  const auto remote_error = GetLastError();
  if (remote_error != ERROR_INVALID_FUNCTION &&
      remote_error != ERROR_NOT_SUPPORTED &&
      remote_error != ERROR_INVALID_PARAMETER) {
    fail("A fixed-local authority path remote status was inconclusive.");
  }
  return authority;
}

std::vector<HeldHandle> attest_path_chain(
    const std::filesystem::path& path,
    std::uint32_t serial) {
  std::vector<HeldHandle> handles;
  auto current = path.root_path();
  handles.push_back(open_attested_path(current, true, serial));
  for (const auto& component : path.relative_path()) {
    current /= component;
    handles.push_back(open_attested_path(current, true, serial));
  }
  return handles;
}

HeldHandle create_attested_directory(
    const std::filesystem::path& path,
    std::uint32_t serial,
    bool must_be_fresh) {
  if (CreateDirectoryW(path.c_str(), nullptr) == 0) {
    if (must_be_fresh || GetLastError() != ERROR_ALREADY_EXISTS) {
      fail("A fixed native verification directory could not be created.");
    }
  }
  return open_attested_path(path, true, serial);
}

std::wstring sha256_handle(HANDLE file) {
  BCRYPT_ALG_HANDLE algorithm = nullptr;
  BCRYPT_HASH_HANDLE hash = nullptr;
  DWORD object_bytes = 0;
  DWORD copied = 0;
  if (BCryptOpenAlgorithmProvider(
          &algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0) < 0 ||
      BCryptGetProperty(
          algorithm,
          BCRYPT_OBJECT_LENGTH,
          reinterpret_cast<PUCHAR>(&object_bytes),
          sizeof(object_bytes),
          &copied,
          0) < 0 ||
      copied != sizeof(object_bytes)) {
    if (algorithm != nullptr) BCryptCloseAlgorithmProvider(algorithm, 0);
    fail("The SHA-256 provider failed closed.");
  }
  std::vector<std::uint8_t> object(object_bytes);
  if (BCryptCreateHash(
          algorithm, &hash, object.data(), object_bytes, nullptr, 0, 0) < 0) {
    BCryptCloseAlgorithmProvider(algorithm, 0);
    fail("The SHA-256 state failed closed.");
  }
  LARGE_INTEGER start{};
  if (SetFilePointerEx(file, start, nullptr, FILE_BEGIN) == 0) {
    BCryptDestroyHash(hash);
    BCryptCloseAlgorithmProvider(algorithm, 0);
    fail("The SHA-256 input could not be rewound.");
  }
  std::array<std::uint8_t, 64 * 1024> buffer{};
  while (true) {
    DWORD read = 0;
    if (ReadFile(file, buffer.data(), static_cast<DWORD>(buffer.size()), &read, nullptr) == 0) {
      BCryptDestroyHash(hash);
      BCryptCloseAlgorithmProvider(algorithm, 0);
      fail("The SHA-256 input read failed closed.");
    }
    if (read == 0) break;
    if (BCryptHashData(hash, buffer.data(), read, 0) < 0) {
      BCryptDestroyHash(hash);
      BCryptCloseAlgorithmProvider(algorithm, 0);
      fail("The SHA-256 update failed closed.");
    }
  }
  std::array<std::uint8_t, 32> digest{};
  if (BCryptFinishHash(hash, digest.data(), static_cast<ULONG>(digest.size()), 0) < 0) {
    BCryptDestroyHash(hash);
    BCryptCloseAlgorithmProvider(algorithm, 0);
    fail("The SHA-256 finalization failed closed.");
  }
  BCryptDestroyHash(hash);
  BCryptCloseAlgorithmProvider(algorithm, 0);
  std::wostringstream output;
  output << std::uppercase << std::hex << std::setfill(L'0');
  for (const auto byte : digest) {
    output << std::setw(2) << static_cast<unsigned>(byte);
  }
  return output.str();
}

std::wstring sha256_file(const std::filesystem::path& path) {
  HeldHandle file(CreateFileW(
      path.c_str(),
      GENERIC_READ,
      FILE_SHARE_READ,
      nullptr,
      OPEN_EXISTING,
      FILE_FLAG_SEQUENTIAL_SCAN,
      nullptr));
  if (!file.valid()) fail("A SHA-256 evidence input could not be opened.");
  return sha256_handle(file.get());
}

std::uint64_t file_bytes(HANDLE file) {
  LARGE_INTEGER size{};
  if (GetFileSizeEx(file, &size) == 0 || size.QuadPart < 0) {
    fail("An exact candidate file size is unavailable.");
  }
  return static_cast<std::uint64_t>(size.QuadPart);
}

bool safe_relative_path(std::wstring_view value) {
  if (value.empty() || value.starts_with(L"\\") || value.starts_with(L"/") ||
      value.find(L':') != std::wstring_view::npos) {
    return false;
  }
  const std::filesystem::path path(value);
  return !path.is_absolute() && std::ranges::none_of(path, [](const auto& part) {
    return part == L".." || part == L"." || part.empty();
  });
}

void write_all(HANDLE file, std::span<const std::uint8_t> bytes) {
  while (!bytes.empty()) {
    DWORD written = 0;
    const auto chunk = static_cast<DWORD>(std::min<std::size_t>(
        bytes.size(), static_cast<std::size_t>(std::numeric_limits<DWORD>::max())));
    if (WriteFile(file, bytes.data(), chunk, &written, nullptr) == 0 || written == 0) {
      fail("The exact staged package write failed closed.");
    }
    bytes = bytes.subspan(written);
  }
}

HeldHandle copy_and_attest_candidate_file(
    const std::filesystem::path& source,
    const std::filesystem::path& destination,
    const CandidateFile& expected,
    std::uint32_t serial) {
  HeldHandle input(CreateFileW(
      source.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
      FILE_FLAG_SEQUENTIAL_SCAN, nullptr));
  if (!input.valid() || file_bytes(input.get()) != expected.bytes ||
      sha256_handle(input.get()) != expected.sha256) {
    fail("The exact candidate package input drifted.");
  }
  LARGE_INTEGER start{};
  if (SetFilePointerEx(input.get(), start, nullptr, FILE_BEGIN) == 0) {
    fail("The exact candidate package input could not be rewound.");
  }
  HeldHandle output(CreateFileW(
      destination.c_str(),
      GENERIC_WRITE | GENERIC_READ,
      FILE_SHARE_READ,
      nullptr,
      CREATE_NEW,
      FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH,
      nullptr));
  if (!output.valid()) fail("The exact staged package destination could not be created.");
  std::array<std::uint8_t, 64 * 1024> buffer{};
  std::uint64_t copied = 0;
  while (true) {
    DWORD read = 0;
    if (ReadFile(input.get(), buffer.data(), static_cast<DWORD>(buffer.size()), &read, nullptr) == 0) {
      fail("The exact candidate package copy read failed closed.");
    }
    if (read == 0) break;
    write_all(output.get(), std::span<const std::uint8_t>(buffer.data(), read));
    copied += read;
  }
  if (copied != expected.bytes || FlushFileBuffers(output.get()) == 0 ||
      sha256_handle(output.get()) != expected.sha256) {
    fail("The exact staged package bytes failed post-copy verification.");
  }
  output.reset();
  auto authority = open_attested_path(destination, false, serial);
  if (file_bytes(authority.get()) != expected.bytes ||
      sha256_handle(authority.get()) != expected.sha256) {
    fail("The exact staged package authority drifted after copy.");
  }
  return authority;
}

std::uint64_t secure_random_u64() {
  std::uint64_t value = 0;
  if (BCryptGenRandom(
          nullptr,
          reinterpret_cast<PUCHAR>(&value),
          sizeof(value),
          BCRYPT_USE_SYSTEM_PREFERRED_RNG) < 0 ||
      value == 0) {
    fail("The native verification run identity is unavailable.");
  }
  return value;
}

std::wstring hex16(std::uint64_t value) {
  std::wostringstream output;
  output << std::hex << std::nouppercase << std::setw(16) << std::setfill(L'0')
         << value;
  return output.str();
}

}  // namespace

namespace {

struct Endpoint final {
  DWORD process_id{};
  std::string protocol;
  std::string family;
  std::string local_address;
  std::uint16_t local_port{};
  std::string remote_address;
  std::uint16_t remote_port{};
  std::uint32_t state{};
  bool external{};

  auto key() const {
    return std::tuple{
        process_id, protocol, family, local_address, local_port,
        remote_address, remote_port, state, external};
  }
};

struct ObservedProcess final {
  DWORD process_id{};
  DWORD parent_process_id{};
  std::string image_name;
  std::string executable_sha256;
};

struct ExternalObservation final {
  std::map<DWORD, ObservedProcess> processes;
  std::map<decltype(Endpoint{}.key()), Endpoint> endpoints;
  std::filesystem::path runtime_path;
  std::wstring runtime_sha256;
  std::string runtime_version;
  std::size_t runtime_process_count{};
  std::size_t snapshot_count{};
  std::vector<HeldHandle> runtime_authorities;
};

struct StagedCandidate final {
  std::filesystem::path local_app_data;
  std::filesystem::path application_root;
  std::filesystem::path verification_root;
  std::filesystem::path package_root;
  VolumeAttestation volume;
  std::vector<HeldHandle> authorities;
};

struct EvidenceFile final {
  std::string path;
  std::uint64_t bytes{};
  std::string sha256;
};

struct VerifiedReplayEvidence final {
  std::string controlled_input_sha256;
  std::string deterministic_output_sha256;
  std::string runtime_replay_sha256;
  std::string canonical_replay_sha256;
  std::uint64_t event_count{};
  std::uint64_t boundary_count{};
};

const CandidateFile& candidate_file(std::wstring_view relative);
void safe_remove_tree(
    const std::filesystem::path& path,
    const std::filesystem::path& allowed_parent,
    std::uint32_t serial);
StagedCandidate stage_candidate(
    const std::filesystem::path& launcher_directory,
    std::vector<std::string>& transcript);
bool valid_verification_project_name(std::string_view value);
void capture_external_observation(
    HANDLE process_job,
    HANDLE completion_port,
    HANDLE root_handle,
    DWORD root_process,
    DWORD notification_wait_milliseconds,
    std::uint32_t system_volume_serial,
    ExternalObservation& observation);
std::string iso_utc_now();
std::string narrow_ascii(std::wstring_view value);
std::string json_escape(std::string_view value);
std::string json_string_value(std::string_view source, std::string_view key);
std::uint64_t json_positive_integer_value(
    std::string_view source,
    std::string_view key);
std::string read_text_file(
    const std::filesystem::path& path,
    std::uint64_t maximum = 32ULL * 1024ULL * 1024ULL);
VerifiedReplayEvidence validate_raw_manifest(std::string_view raw);
void copy_evidence_file(
    const std::filesystem::path& source,
    const std::filesystem::path& destination);
void write_text_file(
    const std::filesystem::path& path,
    std::string_view value);
std::string process_evidence_json(const ExternalObservation& observation);
EvidenceFile evidence_file(
    const std::filesystem::path& directory,
    std::string path);
std::string manifest_json(
    const StagedCandidate& staged,
    const ExternalObservation& observation,
    const VerifiedReplayEvidence& replay,
    std::span<const EvidenceFile> evidence_files,
    std::string_view result,
    bool project_removed,
    bool user_data_removed,
    bool package_removed,
    std::size_t external_attempt_count,
    DWORD shell_exit_code,
    std::string_view started_at,
    std::string_view completed_at,
    std::string_view raw_sha256,
    std::string_view netlog_sha256,
    std::string_view process_sha256,
    std::string_view launcher_sha256);

struct LaunchedProcess final {
  HeldHandle job;
  HeldHandle completion_port;
  HeldHandle process;
  DWORD process_id{};
};

LaunchedProcess launch_exact_shell(const StagedCandidate& staged) {
  const auto shell = staged.package_root / L"GovsPLC.exe";
  const auto& expected_shell = candidate_file(L"GovsPLC.exe");
  auto shell_authority = open_attested_path(shell, false, staged.volume.serial);
  if (file_bytes(shell_authority.get()) != expected_shell.bytes ||
      sha256_handle(shell_authority.get()) != expected_shell.sha256) {
    fail("The exact staged shell identity drifted before launch.");
  }

  HeldHandle job(CreateJobObjectW(nullptr, nullptr));
  if (!job.valid()) fail("The native verification process job could not be created.");
  HeldHandle completion_port(CreateIoCompletionPort(
      INVALID_HANDLE_VALUE, nullptr, 0, 1));
  if (!completion_port.valid()) {
    fail("The native verification process notification port could not be created.");
  }
  JOBOBJECT_ASSOCIATE_COMPLETION_PORT association{};
  association.CompletionKey = job.get();
  association.CompletionPort = completion_port.get();
  if (SetInformationJobObject(
          job.get(),
          JobObjectAssociateCompletionPortInformation,
          &association,
          sizeof(association)) == 0) {
    fail("The native verification process notification port failed closed.");
  }
  JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits{};
  limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
  if (SetInformationJobObject(
          job.get(),
          JobObjectExtendedLimitInformation,
          &limits,
          sizeof(limits)) == 0) {
    fail("The native verification process job failed closed.");
  }

  const std::wstring command_line =
      L"\"" + shell.wstring() + L"\" --verify-native-bridge";
  std::vector<wchar_t> mutable_command(command_line.begin(), command_line.end());
  mutable_command.push_back(L'\0');
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  PROCESS_INFORMATION process{};
  if (CreateProcessW(
          shell.c_str(),
          mutable_command.data(),
          nullptr,
          nullptr,
          FALSE,
          CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
          nullptr,
          staged.package_root.c_str(),
          &startup,
          &process) == 0) {
    fail("The exact staged shell could not be launched.");
  }
  HeldHandle process_handle(process.hProcess);
  HeldHandle thread_handle(process.hThread);
  if (AssignProcessToJobObject(job.get(), process_handle.get()) == 0) {
    TerminateProcess(process_handle.get(), 1);
    fail("The exact staged shell could not be assigned to its scoped process job.");
  }
  if (ResumeThread(thread_handle.get()) == static_cast<DWORD>(-1)) {
    TerminateJobObject(job.get(), 1);
    fail("The exact staged shell could not be resumed.");
  }
  thread_handle.reset();
  return {
      std::move(job),
      std::move(completion_port),
      std::move(process_handle),
      process.dwProcessId,
  };
}

void clear_fixed_evidence_outputs(const std::filesystem::path& evidence_root) {
  constexpr std::array<std::wstring_view, 8> names{
      L"candidate-package-manifest.json",
      L"native-launcher-transcript.log",
      L"native-netlog.json",
      L"native-network-analysis.json",
      L"native-platform-evidence-manifest.json",
      L"native-platform-observer-manifest.json",
      L"native-process-endpoints.json",
      L"native-run-raw.json",
  };
  for (const auto name : names) {
    const auto path = evidence_root / name;
    if (DeleteFileW(path.c_str()) == 0 &&
        GetLastError() != ERROR_FILE_NOT_FOUND &&
        GetLastError() != ERROR_PATH_NOT_FOUND) {
      fail("A stale fixed native evidence file could not be cleared.");
    }
  }
}

void remove_project_and_user_data(
    const StagedCandidate& staged,
    std::string_view project_name,
    bool& project_removed,
    bool& user_data_removed) {
  if (!valid_verification_project_name(project_name)) {
    fail("The production bridge journey returned an unsafe verification project identity.");
  }
  const auto wide_name = std::filesystem::path(std::string(project_name)).wstring();
  const auto projects = staged.application_root / L"Projects";
  const auto project = projects / wide_name;
  if (GetFileAttributesW(project.c_str()) == INVALID_FILE_ATTRIBUTES) {
    fail("The production bridge journey project was not present for restoration.");
  }
  safe_remove_tree(project, projects, staged.volume.serial);
  project_removed = GetFileAttributesW(project.c_str()) == INVALID_FILE_ATTRIBUTES &&
      GetLastError() == ERROR_FILE_NOT_FOUND;

  constexpr std::string_view prefix = "Phase-2-Native-";
  const auto session = project_name.substr(prefix.size(), 16);
  const std::filesystem::path user_data =
      staged.application_root / std::filesystem::path("WebView2-" + std::string(session));
  safe_remove_tree(user_data, staged.application_root, staged.volume.serial);
  user_data_removed = GetFileAttributesW(user_data.c_str()) == INVALID_FILE_ATTRIBUTES &&
      (GetLastError() == ERROR_FILE_NOT_FOUND || GetLastError() == ERROR_PATH_NOT_FOUND);
  if (!project_removed || !user_data_removed) {
    fail("The native production bridge journey did not restore its scoped state.");
  }
}

void best_effort_remove_stage(StagedCandidate& staged) noexcept {
  try {
    staged.authorities.clear();
    if (!staged.package_root.empty() && !staged.verification_root.empty()) {
      safe_remove_tree(staged.package_root, staged.verification_root, staged.volume.serial);
    }
  } catch (...) {
  }
}

int run_native_e2e() {
  const auto launcher = executable_path();
  const auto launcher_directory = launcher.parent_path();
  if (_wcsicmp(launcher.filename().c_str(), L"Run-Native-E2E.exe") != 0 ||
      _wcsicmp(launcher_directory.filename().c_str(), L"native-build") != 0 ||
      _wcsicmp(launcher_directory.parent_path().filename().c_str(),
               L".phase2-verification") != 0) {
    fail("The fixed no-argument launcher is outside native-build.");
  }
  const auto evidence_root = launcher_directory.parent_path() / L"native-e2e";
  std::error_code directory_error;
  std::filesystem::create_directories(evidence_root, directory_error);
  if (directory_error) fail("The fixed native evidence directory could not be created.");
  clear_fixed_evidence_outputs(evidence_root);

  HeldHandle mutex(CreateMutexW(
      nullptr, TRUE, L"Local\\GovsPLC-Phase2-Native-E2E-Exact-Candidate"));
  if (!mutex.valid() || GetLastError() == ERROR_ALREADY_EXISTS) {
    fail("Another exact native verification launcher is already running.");
  }
  WSADATA winsock{};
  if (WSAStartup(MAKEWORD(2, 2), &winsock) != 0) {
    fail("The external endpoint observation API could not be initialized.");
  }
  struct WinsockCleanup final {
    ~WinsockCleanup() { WSACleanup(); }
  } winsock_cleanup;
  (void)winsock_cleanup;

  const auto started_at = iso_utc_now();
  std::vector<std::string> transcript{
      "fixed no-argument native product-path verification started",
      "candidate manifest and package inventory matched compile-time SHA-256 bindings",
  };
  std::optional<StagedCandidate> staged;
  try {
    staged.emplace(stage_candidate(launcher_directory, transcript));
    auto launched = launch_exact_shell(*staged);
    transcript.push_back("exact staged GovsPLC.exe launched with sole --verify-native-bridge argument");

    ExternalObservation observation{};
    const auto deadline = GetTickCount64() + kRunTimeoutMilliseconds;
    DWORD wait_status = WAIT_TIMEOUT;
    while (GetTickCount64() < deadline) {
      capture_external_observation(
          launched.job.get(), launched.completion_port.get(), launched.process.get(),
          launched.process_id, 50, staged->volume.serial, observation);
      wait_status = WaitForSingleObject(launched.process.get(), 0);
      if (wait_status == WAIT_OBJECT_0) break;
      if (wait_status != WAIT_TIMEOUT) {
        TerminateJobObject(launched.job.get(), 1);
        fail("The exact staged shell wait failed closed.");
      }
    }
    if (wait_status != WAIT_OBJECT_0) {
      TerminateJobObject(launched.job.get(), 1);
      fail("The exact staged shell exceeded the bounded verification timeout.");
    }
    DWORD shell_exit_code = 0;
    if (GetExitCodeProcess(launched.process.get(), &shell_exit_code) == 0) {
      TerminateJobObject(launched.job.get(), 1);
      fail("The exact staged shell exit status is unavailable.");
    }
    TerminateJobObject(launched.job.get(), 0);
    launched.process.reset();
    launched.job.reset();
    transcript.push_back("shell process exited and the scoped descendant job was terminated");

    const auto raw_source = staged->package_root / L"native-run-raw.json";
    if (GetFileAttributesW(raw_source.c_str()) != INVALID_FILE_ATTRIBUTES) {
      const auto diagnostic_raw = read_text_file(raw_source);
      if (json_string_value(diagnostic_raw, "result") == "FAIL") {
        const auto detail = json_string_value(diagnostic_raw, "error");
        throw std::runtime_error(
            "The exact staged shell failed closed before verification completed: " + detail);
      }
    }

    if (observation.runtime_path.empty() || observation.runtime_sha256.size() != 64 ||
        observation.runtime_version.empty()) {
      throw std::runtime_error(
          "No complete Microsoft Edge WebView2 runtime identity was observed; shellExitCode=" +
          std::to_string(shell_exit_code) +
          " observedProcessCount=" + std::to_string(observation.processes.size()) +
          " snapshotCount=" + std::to_string(observation.snapshot_count));
    }
    const auto external_attempt_count = static_cast<std::size_t>(std::ranges::count_if(
        observation.endpoints, [](const auto& entry) { return entry.second.external; }));

    const auto netlog_source = staged->package_root / L"native-netlog.json";
    const auto raw = read_text_file(raw_source);
    const auto replay = validate_raw_manifest(raw);
    const auto netlog = read_text_file(netlog_source, 256ULL * 1024ULL * 1024ULL);
    if (netlog.size() < 32 || netlog.front() != '{' ||
        netlog.find("\"events\"") == std::string::npos) {
      fail("The Chromium NetLog was missing or incomplete.");
    }
    const auto project_name = json_string_value(raw, "projectName");
    transcript.push_back("raw host bridge manifest and Chromium NetLog validated");

    const auto source_manifest =
        launcher_directory / L"package" / L"candidate-package-manifest.json";
    copy_evidence_file(
        source_manifest, evidence_root / L"candidate-package-manifest.json");
    copy_evidence_file(raw_source, evidence_root / L"native-run-raw.json");
    copy_evidence_file(netlog_source, evidence_root / L"native-netlog.json");
    // Preserve the exact post-replace project before scoped cleanup. The raw
    // DOM receipt is diagnostic only; an independent verifier needs these
    // committed bytes to recompute the replay rather than trusting the host.
    copy_evidence_file(
        staged->application_root / L"Projects" /
            std::filesystem::path(project_name).wstring(),
        evidence_root / L"native-committed-project.vlabproj");
    write_text_file(
        evidence_root / L"native-process-endpoints.json",
        process_evidence_json(observation));

    bool project_removed = false;
    bool user_data_removed = false;
    staged->authorities.clear();
    remove_project_and_user_data(
        *staged, project_name, project_removed, user_data_removed);
    safe_remove_tree(
        staged->package_root, staged->verification_root, staged->volume.serial);
    const bool package_removed =
        GetFileAttributesW(staged->package_root.c_str()) == INVALID_FILE_ATTRIBUTES &&
        (GetLastError() == ERROR_FILE_NOT_FOUND || GetLastError() == ERROR_PATH_NOT_FOUND);
    if (!package_removed) fail("The exact staged package was not removed after verification.");
    transcript.push_back("scoped project, WebView2 user data, and staged package were removed");

    std::ostringstream transcript_output;
    for (const auto& line : transcript) transcript_output << line << '\n';
    transcript_output << "shellExitCode=" << shell_exit_code << '\n'
                      << "externalAttemptCount=" << external_attempt_count << '\n'
                      << "browserRuntimeProduct=microsoft-edge-webview2\n"
                      << "browserRuntimeVersion=" << observation.runtime_version << '\n';
    write_text_file(
        evidence_root / L"native-launcher-transcript.log",
        transcript_output.str());

    std::array<EvidenceFile, 6> files{
        evidence_file(evidence_root, "candidate-package-manifest.json"),
        evidence_file(evidence_root, "native-committed-project.vlabproj"),
        evidence_file(evidence_root, "native-launcher-transcript.log"),
        evidence_file(evidence_root, "native-netlog.json"),
        evidence_file(evidence_root, "native-process-endpoints.json"),
        evidence_file(evidence_root, "native-run-raw.json"),
    };
    const auto result = kCandidateDevelopmentDirty
        ? "INCONCLUSIVE_DEVELOPMENT"
        : "REQUIRES_BOUND_NETLOG_ANALYSIS";
    const auto completed_at = iso_utc_now();
    const auto raw_hash = narrow_ascii(sha256_file(evidence_root / L"native-run-raw.json"));
    const auto netlog_hash = narrow_ascii(sha256_file(evidence_root / L"native-netlog.json"));
    const auto process_hash = narrow_ascii(
        sha256_file(evidence_root / L"native-process-endpoints.json"));
    const auto launcher_hash = narrow_ascii(sha256_file(launcher));
    write_text_file(
        evidence_root / L"native-platform-observer-manifest.json",
        manifest_json(
            *staged,
            observation,
            replay,
            files,
            result,
            project_removed,
            user_data_removed,
            package_removed,
            external_attempt_count,
            shell_exit_code,
            started_at,
            completed_at,
            raw_hash,
            netlog_hash,
            process_hash,
            launcher_hash));
    staged.reset();
    return 0;
  } catch (...) {
    if (staged.has_value()) best_effort_remove_stage(*staged);
    throw;
  }
}

void write_failure_transcript_best_effort(std::string_view message) noexcept {
  try {
    const auto directory = executable_path().parent_path();
    if (_wcsicmp(directory.filename().c_str(), L"native-build") != 0) return;
    const auto output = directory.parent_path() / L"native-e2e";
    std::error_code error;
    std::filesystem::create_directories(output, error);
    if (error) return;
    std::ofstream log(
        output / L"native-launcher-transcript.log",
        std::ios::binary | std::ios::trunc);
    if (!log) return;
    log << "fixed no-argument native product-path verification failed\n"
        << "error=" << message << '\n';
  } catch (...) {
  }
}

}  // namespace

int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR command_line, int) {
  try {
    if (command_line == nullptr || command_line[0] != L'\0') {
      fail("The native verification launcher accepts no argument or path input.");
    }
    int argument_count = 0;
    LPWSTR* arguments = CommandLineToArgvW(GetCommandLineW(), &argument_count);
    if (arguments == nullptr || argument_count != 1) {
      if (arguments != nullptr) LocalFree(arguments);
      fail("The native verification launcher accepts zero arguments.");
    }
    LocalFree(arguments);
    return run_native_e2e();
  } catch (const std::exception& error) {
    write_failure_transcript_best_effort(error.what());
    MessageBoxA(
        nullptr,
        error.what(),
        "PLC Engineering Simulator - Native Phase 2 Verification",
        MB_OK | MB_ICONERROR | MB_SYSTEMMODAL);
    return 1;
  }
}

namespace {

void delete_handle(HANDLE handle) {
  FILE_DISPOSITION_INFO disposition{};
  disposition.DeleteFile = TRUE;
  if (SetFileInformationByHandle(
          handle,
          FileDispositionInfo,
          &disposition,
          sizeof(disposition)) == 0) {
    fail("A scoped native verification artifact could not be removed.");
  }
}

void safe_remove_tree(
    const std::filesystem::path& path,
    const std::filesystem::path& allowed_parent,
    std::uint32_t serial) {
  const auto normalized = normalized_path(path.wstring());
  const auto parent = normalized_path(allowed_parent.wstring());
  if (!normalized.starts_with(parent + L"\\") || normalized == parent) {
    fail("A scoped cleanup target escaped its exact fixed parent.");
  }
  const auto attributes = GetFileAttributesW(path.c_str());
  if (attributes == INVALID_FILE_ATTRIBUTES) {
    if (GetLastError() == ERROR_FILE_NOT_FOUND ||
        GetLastError() == ERROR_PATH_NOT_FOUND) {
      return;
    }
    fail("A scoped cleanup target could not be inspected.");
  }
  if ((attributes & kUnsafeBackingAttributes) != 0) {
    fail("A scoped cleanup target became provider-backed or redirected.");
  }
  if ((attributes & FILE_ATTRIBUTE_DIRECTORY) == 0) {
    auto file = open_attested_path(path, false, serial, DELETE);
    delete_handle(file.get());
    return;
  }

  WIN32_FIND_DATAW entry{};
  HeldFind search(FindFirstFileW((path / L"*").c_str(), &entry));
  if (!search.valid()) fail("A scoped cleanup directory could not be enumerated.");
  do {
    const std::wstring_view name(entry.cFileName);
    if (name == L"." || name == L"..") continue;
    if (name.empty() || name.find(L'\\') != std::wstring_view::npos ||
        name.find(L'/') != std::wstring_view::npos ||
        name.find(L':') != std::wstring_view::npos) {
      fail("A scoped cleanup directory contained an unsafe entry name.");
    }
    safe_remove_tree(path / std::filesystem::path(name), allowed_parent, serial);
  } while (FindNextFileW(search.get(), &entry) != 0);
  if (GetLastError() != ERROR_NO_MORE_FILES) {
    fail("A scoped cleanup directory enumeration was incomplete.");
  }
  search.reset();
  auto directory = open_attested_path(path, true, serial, DELETE);
  delete_handle(directory.get());
}

const CandidateFile& candidate_file(std::wstring_view relative) {
  const auto found = std::ranges::find_if(kCandidateFiles, [relative](const auto& row) {
    return row.relative_path == relative;
  });
  if (found == std::end(kCandidateFiles)) {
    fail("The generated candidate inventory omitted a required file.");
  }
  return *found;
}

StagedCandidate stage_candidate(
    const std::filesystem::path& launcher_directory,
    std::vector<std::string>& transcript) {
  const auto source_package = launcher_directory / L"package";
  const auto source_manifest = source_package / L"candidate-package-manifest.json";
  HeldHandle manifest_input(CreateFileW(
      source_manifest.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr,
      OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, nullptr));
  if (!manifest_input.valid() ||
      file_bytes(manifest_input.get()) != kCandidateManifestBytes ||
      sha256_handle(manifest_input.get()) != kCandidateManifestSha256) {
    fail("The compile-time-bound candidate manifest drifted.");
  }
  for (const auto& row : kCandidateFiles) {
    if (!safe_relative_path(row.relative_path) || row.bytes == 0 ||
        row.sha256.size() != 64) {
      fail("The compile-time-bound candidate package inventory is invalid.");
    }
    HeldHandle source(CreateFileW(
        (source_package / std::filesystem::path(row.relative_path)).c_str(),
        GENERIC_READ,
        FILE_SHARE_READ,
        nullptr,
        OPEN_EXISTING,
        FILE_FLAG_SEQUENTIAL_SCAN,
        nullptr));
    if (!source.valid() || file_bytes(source.get()) != row.bytes ||
        sha256_handle(source.get()) != row.sha256) {
      fail("A compile-time-bound candidate package file drifted.");
    }
  }

  StagedCandidate staged{};
  staged.local_app_data = authoritative_local_app_data();
  staged.volume = attest_native_system_volume(staged.local_app_data);
  staged.authorities = attest_path_chain(staged.local_app_data, staged.volume.serial);
  staged.application_root = staged.local_app_data / L"GovsPLC";
  staged.authorities.push_back(create_attested_directory(
      staged.application_root, staged.volume.serial, false));
  staged.verification_root = staged.application_root / L"NativeVerification";
  staged.authorities.push_back(create_attested_directory(
      staged.verification_root, staged.volume.serial, false));

  const auto prefix = std::wstring(kCandidateManifestSha256).substr(0, 16);
  for (unsigned attempt = 0; attempt < 8; ++attempt) {
    staged.package_root = staged.verification_root /
        (L"Candidate-" + prefix + L"-" + hex16(secure_random_u64()));
    if (CreateDirectoryW(staged.package_root.c_str(), nullptr) != 0) break;
    if (GetLastError() != ERROR_ALREADY_EXISTS || attempt == 7) {
      fail("A fresh exact-candidate staging directory could not be created.");
    }
  }
  staged.authorities.push_back(open_attested_path(
      staged.package_root, true, staged.volume.serial));
  for (const auto name : {L"app", L"third-party"}) {
    staged.authorities.push_back(create_attested_directory(
        staged.package_root / name, staged.volume.serial, true));
  }

  for (const auto& row : kCandidateFiles) {
    staged.authorities.push_back(copy_and_attest_candidate_file(
        source_package / std::filesystem::path(row.relative_path),
        staged.package_root / std::filesystem::path(row.relative_path),
        row,
        staged.volume.serial));
  }
  const CandidateFile manifest_row{
      L"candidate-package-manifest.json",
      kCandidateManifestBytes,
      kCandidateManifestSha256,
  };
  staged.authorities.push_back(copy_and_attest_candidate_file(
      source_manifest,
      staged.package_root / std::filesystem::path(manifest_row.relative_path),
      manifest_row,
      staged.volume.serial));
  transcript.push_back("candidate package staged on attested native system storage");
  return staged;
}

std::string process_evidence_json(const ExternalObservation& observation) {
  std::ostringstream output;
  output << "{\n"
         << "  \"schemaVersion\": \"1.0\",\n"
         << "  \"evidenceKind\": \"WINDOWS_NATIVE_EXTERNAL_PROCESS_ENDPOINT_CAPTURE\",\n"
         << "  \"capturedAt\": \"" << iso_utc_now() << "\",\n"
         << "  \"snapshotIntervalMilliseconds\": 50,\n"
         << "  \"snapshotCount\": " << observation.snapshot_count << ",\n"
         << "  \"captureComplete\": true,\n"
         << "  \"processes\": [\n";
  std::size_t process_index = 0;
  for (const auto& [process_id, process] : observation.processes) {
    (void)process_id;
    output << "    {\"executableSha256\": \""
           << json_escape(process.executable_sha256)
           << "\", \"imageName\": \"" << json_escape(process.image_name)
           << "\", \"parentProcessId\": " << process.parent_process_id
           << ", \"processId\": " << process.process_id << "}";
    output << (++process_index == observation.processes.size() ? "\n" : ",\n");
  }
  output << "  ],\n  \"endpoints\": [\n";
  std::size_t endpoint_index = 0;
  for (const auto& [key, endpoint] : observation.endpoints) {
    (void)key;
    output << "    {\"external\": " << (endpoint.external ? "true" : "false")
           << ", \"family\": \"" << endpoint.family
           << "\", \"localAddress\": \"" << json_escape(endpoint.local_address)
           << "\", \"localPort\": " << endpoint.local_port
           << ", \"processId\": " << endpoint.process_id
           << ", \"protocol\": \"" << endpoint.protocol
           << "\", \"remoteAddress\": \"" << json_escape(endpoint.remote_address)
           << "\", \"remotePort\": " << endpoint.remote_port
           << ", \"state\": " << endpoint.state << "}";
    output << (++endpoint_index == observation.endpoints.size() ? "\n" : ",\n");
  }
  const auto external_count = std::ranges::count_if(
      observation.endpoints, [](const auto& entry) { return entry.second.external; });
  output << "  ],\n"
         << "  \"externalAttemptCount\": " << external_count << ",\n"
         << "  \"runtimeProcessObservationCount\": "
         << observation.runtime_process_count << "\n"
         << "}\n";
  return output.str();
}

EvidenceFile evidence_file(
    const std::filesystem::path& directory,
    std::string path) {
  const auto file = directory / std::filesystem::path(path);
  HeldHandle handle(CreateFileW(
      file.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
      FILE_FLAG_SEQUENTIAL_SCAN, nullptr));
  if (!handle.valid()) fail("A listed native evidence file is missing.");
  return {
      std::move(path),
      file_bytes(handle.get()),
      narrow_ascii(sha256_handle(handle.get())),
  };
}

void copy_evidence_file(
    const std::filesystem::path& source,
    const std::filesystem::path& destination) {
  if (DeleteFileW(destination.c_str()) == 0 &&
      GetLastError() != ERROR_FILE_NOT_FOUND &&
      GetLastError() != ERROR_PATH_NOT_FOUND) {
    fail("A stale fixed evidence output could not be removed.");
  }
  if (CopyFileW(source.c_str(), destination.c_str(), TRUE) == 0) {
    fail("A fixed native evidence file could not be copied.");
  }
  if (sha256_file(source) != sha256_file(destination)) {
    fail("A copied native evidence file failed SHA-256 verification.");
  }
}

std::string manifest_json(
    const StagedCandidate& staged,
    const ExternalObservation& observation,
    const VerifiedReplayEvidence& replay,
    std::span<const EvidenceFile> evidence_files,
    std::string_view result,
    bool project_removed,
    bool user_data_removed,
    bool package_removed,
    std::size_t external_attempt_count,
    DWORD shell_exit_code,
    std::string_view started_at,
    std::string_view completed_at,
    std::string_view raw_sha256,
    std::string_view netlog_sha256,
    std::string_view process_sha256,
    std::string_view launcher_sha256) {
  std::ostringstream output;
  output << "{\n"
         << "  \"schemaVersion\": \"1.0\",\n"
         << "  \"evidenceKind\": \"WINDOWS_NATIVE_PRODUCT_PATH_OBSERVER_MANIFEST\",\n"
         << "  \"result\": \"" << result << "\",\n"
         << "  \"startedAt\": \"" << started_at << "\",\n"
         << "  \"completedAt\": \"" << completed_at << "\",\n"
         << "  \"candidateCommit\": \"" << narrow_ascii(kCandidateCommit) << "\",\n"
         << "  \"candidateTree\": \"" << narrow_ascii(kCandidateTree) << "\",\n"
         << "  \"candidateDevelopmentDirty\": "
         << (kCandidateDevelopmentDirty ? "true" : "false") << ",\n"
         << "  \"candidateManifestSha256\": \""
         << narrow_ascii(kCandidateManifestSha256) << "\",\n"
         << "  \"candidatePackageContractSha256\": \""
         << narrow_ascii(kCandidatePackageContractSha256) << "\",\n"
         << "  \"reviewedRequirementMappingSha256\": \""
         << narrow_ascii(kReviewedRequirementMappingSha256) << "\",\n"
         << "  \"launcherExecutableSha256\": \"" << launcher_sha256 << "\",\n"
         << "  \"launcherSourceSha256\": \""
         << narrow_ascii(kLauncherSourceSha256) << "\",\n"
         << "  \"launcherBuildScriptSha256\": \""
         << narrow_ascii(kLauncherBuildScriptSha256) << "\",\n"
         << "  \"nativeEvidenceFinalizerSha256\": \""
         << narrow_ascii(kNativeEvidenceFinalizerSha256) << "\",\n"
         << "  \"isolationAnalysisLibrarySha256\": \""
         << narrow_ascii(kIsolationAnalysisLibrarySha256) << "\",\n"
         << "  \"productionPathExercised\": true,\n"
         << "  \"shellExitCode\": " << shell_exit_code << ",\n"
         << "  \"runtimeProductIdentity\": \"Microsoft Edge WebView2 Runtime\",\n"
         << "  \"browserRuntimeProduct\": \"microsoft-edge-webview2\",\n"
         << "  \"browserRuntimeVersion\": \""
         << json_escape(observation.runtime_version) << "\",\n"
         << "  \"browserExecutableSha256\": \""
         << narrow_ascii(observation.runtime_sha256) << "\",\n"
         << "  \"runtimeBackingAttested\": "
         << (!observation.runtime_authorities.empty() ? "true" : "false") << ",\n"
         << "  \"runtimeReplaySha256\": \""
         << replay.runtime_replay_sha256 << "\",\n"
         << "  \"canonicalReplaySha256\": \""
         << replay.canonical_replay_sha256 << "\",\n"
         << "  \"controlledInputSha256\": \""
         << replay.controlled_input_sha256 << "\",\n"
         << "  \"deterministicOutputSha256\": \""
         << replay.deterministic_output_sha256 << "\",\n"
         << "  \"verifiedReplayEventCount\": "
         << replay.event_count << ",\n"
         << "  \"verifiedReplayBoundaryCount\": "
         << replay.boundary_count << ",\n"
         << "  \"instrumentationStatus\": \"REQUIRES_BOUND_NETLOG_ANALYSIS\",\n"
         << "  \"instrumentationComplete\": false,\n"
         << "  \"endpointSnapshotIntervalMilliseconds\": 50,\n"
         << "  \"endpointSnapshotCount\": " << observation.snapshot_count << ",\n"
         << "  \"endpointSnapshotExternalObservationCount\": "
         << external_attempt_count << ",\n"
         << "  \"externalAttemptCount\": " << external_attempt_count << ",\n"
         << "  \"zeroExternalAttempts\": false,\n"
         << "  \"rawHostManifestSha256\": \"" << raw_sha256 << "\",\n"
         << "  \"chromiumNetLogSha256\": \"" << netlog_sha256 << "\",\n"
         << "  \"externalProcessEvidenceSha256\": \"" << process_sha256 << "\",\n"
         << "  \"fixedNativeBackingAttestation\": {\n"
         << "    \"authoritativeKnownFolder\": \"FOLDERID_LocalAppData\",\n"
         << "    \"nativeSystemVolume\": true,\n"
         << "    \"fixedDrive\": true,\n"
         << "    \"filesystem\": \"" << narrow_ascii(staged.volume.file_system) << "\",\n"
         << "    \"storageBus\": \"" << narrow_ascii(staged.volume.storage_bus) << "\",\n"
         << "    \"providerBacked\": false,\n"
         << "    \"remote\": false,\n"
         << "    \"removable\": false,\n"
         << "    \"hotplug\": false,\n"
         << "    \"special\": false,\n"
         << "    \"redirected\": false\n"
         << "  },\n"
         << "  \"restoration\": {\n"
         << "    \"projectRemoved\": " << (project_removed ? "true" : "false") << ",\n"
         << "    \"userDataRemoved\": " << (user_data_removed ? "true" : "false") << ",\n"
         << "    \"stagedPackageRemoved\": " << (package_removed ? "true" : "false") << "\n"
         << "  },\n"
         << "  \"evidenceFiles\": [\n";
  for (std::size_t index = 0; index < evidence_files.size(); ++index) {
    const auto& row = evidence_files[index];
    output << "    {\"bytes\": " << row.bytes << ", \"path\": \""
           << json_escape(row.path) << "\", \"sha256\": \""
           << row.sha256 << "\"}"
           << (index + 1 == evidence_files.size() ? "\n" : ",\n");
  }
  output << "  ]\n}\n";
  return output.str();
}

}  // namespace

namespace {

std::string narrow_ascii(std::wstring_view value) {
  std::string output;
  output.reserve(value.size());
  for (const auto character : value) {
    if (character < 0x20 || character > 0x7e) {
      fail("An evidence identity was not bounded ASCII.");
    }
    output.push_back(static_cast<char>(character));
  }
  return output;
}

std::string json_escape(std::string_view value) {
  std::ostringstream output;
  for (const unsigned char byte : value) {
    switch (byte) {
      case '"': output << "\\\""; break;
      case '\\': output << "\\\\"; break;
      case '\b': output << "\\b"; break;
      case '\f': output << "\\f"; break;
      case '\n': output << "\\n"; break;
      case '\r': output << "\\r"; break;
      case '\t': output << "\\t"; break;
      default:
        if (byte < 0x20) {
          output << "\\u00" << std::uppercase << std::hex << std::setw(2)
                 << std::setfill('0') << static_cast<unsigned>(byte)
                 << std::dec;
        } else {
          output << static_cast<char>(byte);
        }
    }
  }
  return output.str();
}

std::string iso_utc_now() {
  SYSTEMTIME value{};
  GetSystemTime(&value);
  std::ostringstream output;
  output << std::setfill('0') << std::setw(4) << value.wYear << '-'
         << std::setw(2) << value.wMonth << '-' << std::setw(2) << value.wDay
         << 'T' << std::setw(2) << value.wHour << ':' << std::setw(2)
         << value.wMinute << ':' << std::setw(2) << value.wSecond << '.'
         << std::setw(3) << value.wMilliseconds << 'Z';
  return output.str();
}

std::vector<std::uint8_t> read_file_bytes(
    const std::filesystem::path& path,
    std::uint64_t maximum = 256ULL * 1024ULL * 1024ULL) {
  HeldHandle file(CreateFileW(
      path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
      FILE_FLAG_SEQUENTIAL_SCAN, nullptr));
  if (!file.valid()) {
    fail("A required native evidence file could not be opened; path=" +
         path.string() + ", win32=" + std::to_string(GetLastError()) + ".");
  }
  const auto size = file_bytes(file.get());
  if (size == 0 || size > maximum ||
      size > static_cast<std::uint64_t>(std::numeric_limits<std::size_t>::max())) {
    fail("A required native evidence file has an invalid bounded size.");
  }
  std::vector<std::uint8_t> result(static_cast<std::size_t>(size));
  std::size_t offset = 0;
  while (offset < result.size()) {
    DWORD read = 0;
    const auto remaining = result.size() - offset;
    const auto chunk = static_cast<DWORD>(std::min<std::size_t>(
        remaining, static_cast<std::size_t>(std::numeric_limits<DWORD>::max())));
    if (ReadFile(file.get(), result.data() + offset, chunk, &read, nullptr) == 0 ||
        read == 0) {
      fail("A required native evidence file read failed closed.");
    }
    offset += read;
  }
  return result;
}

std::string read_text_file(
    const std::filesystem::path& path,
    std::uint64_t maximum) {
  const auto bytes = read_file_bytes(path, maximum);
  return std::string(reinterpret_cast<const char*>(bytes.data()), bytes.size());
}

void write_text_file(
    const std::filesystem::path& path,
    std::string_view value) {
  std::ofstream output(path, std::ios::binary | std::ios::trunc);
  if (!output) fail("The native evidence output could not be created.");
  output.write(value.data(), static_cast<std::streamsize>(value.size()));
  output.flush();
  if (!output) fail("The native evidence output write failed closed.");
}

std::size_t unique_json_key_offset(
    std::string_view source,
    std::string_view key) {
  const std::string needle = "\"" + std::string(key) + "\"";
  const auto offset = source.find(needle);
  if (offset == std::string_view::npos) {
    fail("A raw native manifest field is missing.");
  }
  if (source.find(needle, offset + needle.size()) != std::string_view::npos) {
    fail("A raw native manifest field is duplicated.");
  }
  return offset;
}

std::size_t json_value_offset(
    std::string_view source,
    std::string_view key) {
  const std::string needle = "\"" + std::string(key) + "\"";
  auto offset = unique_json_key_offset(source, key);
  offset = source.find(':', offset + needle.size());
  if (offset == std::string_view::npos) fail("A raw native manifest field is malformed.");
  ++offset;
  while (offset < source.size() &&
         (source[offset] == ' ' || source[offset] == '\t' ||
          source[offset] == '\r' || source[offset] == '\n')) {
    ++offset;
  }
  if (offset >= source.size()) fail("A raw native manifest value is missing.");
  return offset;
}

std::string json_string_value(std::string_view source, std::string_view key) {
  auto offset = json_value_offset(source, key);
  if (offset >= source.size() || source[offset] != '"') {
    fail("A raw native manifest string field is malformed.");
  }
  ++offset;
  std::string value;
  while (offset < source.size()) {
    const auto byte = source[offset++];
    if (byte == '"') return value;
    if (byte == '\\') {
      if (offset >= source.size()) fail("A raw native manifest escape is malformed.");
      const auto escaped = source[offset++];
      if (escaped != '"' && escaped != '\\' && escaped != '/') {
        fail("A raw native manifest string contains an unsupported escape.");
      }
      value.push_back(escaped);
    } else if (static_cast<unsigned char>(byte) < 0x20) {
      fail("A raw native manifest string contains a control byte.");
    } else {
      value.push_back(byte);
    }
  }
  fail("A raw native manifest string is unterminated.");
}

std::uint64_t json_positive_integer_value(
    std::string_view source,
    std::string_view key) {
  auto offset = json_value_offset(source, key);
  if (source[offset] < '0' || source[offset] > '9' ||
      (source[offset] == '0' && offset + 1 < source.size() &&
       source[offset + 1] >= '0' && source[offset + 1] <= '9')) {
    fail("A raw native manifest integer field is not canonical decimal.");
  }
  std::uint64_t value = 0;
  while (offset < source.size() && source[offset] >= '0' && source[offset] <= '9') {
    const auto digit = static_cast<std::uint64_t>(source[offset] - '0');
    if (value > (1'000'000ULL - digit) / 10ULL) {
      fail("A raw native manifest integer field exceeds its fixed bound.");
    }
    value = value * 10ULL + digit;
    ++offset;
  }
  while (offset < source.size() &&
         (source[offset] == ' ' || source[offset] == '\t' ||
          source[offset] == '\r' || source[offset] == '\n')) {
    ++offset;
  }
  if (value == 0 || offset >= source.size() ||
      (source[offset] != ',' && source[offset] != '}')) {
    fail("A raw native manifest integer field is malformed or not positive.");
  }
  return value;
}

bool valid_sha256_ascii(std::string_view value) {
  return value.size() == 64 && std::ranges::all_of(value, [](char byte) {
    return (byte >= '0' && byte <= '9') || (byte >= 'A' && byte <= 'F');
  });
}

VerifiedReplayEvidence validate_raw_manifest(std::string_view raw) {
  const std::array<std::string_view, 18> required{
      "\"schemaVersion\": \"1.0\"",
      "\"evidenceKind\": \"WINDOWS_NATIVE_BRIDGE_RAW_RUN\"",
      "\"result\": \"PASS\"",
      "\"fixedLocalBacking\": true",
      "\"providerBacked\": false",
      "\"remote\": false",
      "\"removable\": false",
      "\"special\": false",
      "\"redirected\": false",
      "\"metadataOnlyBeforeAcceptance\": true",
      "\"selectedByteIoBeforeAcceptance\": false",
      "\"verificationStage\": 4",
      "\"operations\": [\"create\", \"open\", \"replace\"]",
      "\"verificationJourneyId\": \"govs.native-runnable-hardware-replay/v4\"",
      "\"verificationUuidVersion\": \"govs-p2-native-verification-uuid-v1\"",
      "\"verificationUuidSeed\": \"2B42B846-54D0-4C61-9B72-4CD3AFC50001\"",
      "\"verificationUuidOrdinalStart\": 1",
      "\"verificationUuidOrdinalContract\": \"after-saved-document:build=4,power=5,preview=6,commit=7,online=8,run=9,scan=10,stop=11,capture=12\"",
  };
  if (std::ranges::any_of(required, [raw](std::string_view value) {
        return raw.find(value) == std::string_view::npos;
      }) ||
      json_string_value(raw, "instrumentationStatus") != "REQUIRES_EXTERNAL_HARNESS") {
    fail("The native shell raw run did not complete the production bridge journey.");
  }
  VerifiedReplayEvidence replay{
      json_string_value(raw, "controlledInputSha256"),
      json_string_value(raw, "deterministicOutputSha256"),
      json_string_value(raw, "runtimeReplaySha256"),
      json_string_value(raw, "canonicalReplaySha256"),
      json_positive_integer_value(raw, "verifiedReplayEventCount"),
      json_positive_integer_value(raw, "verifiedReplayBoundaryCount"),
  };
  if (!valid_sha256_ascii(replay.controlled_input_sha256) ||
      !valid_sha256_ascii(replay.deterministic_output_sha256) ||
      !valid_sha256_ascii(replay.runtime_replay_sha256) ||
      !valid_sha256_ascii(replay.canonical_replay_sha256)) {
    fail("The controlled input, deterministic output, or replay hashes are not canonical SHA-256 values.");
  }
  return replay;
}

bool valid_verification_project_name(std::string_view value) {
  constexpr std::string_view prefix = "Phase-2-Native-";
  constexpr std::string_view suffix = ".vlabproj";
  if (!value.starts_with(prefix) || !value.ends_with(suffix) ||
      value.size() != prefix.size() + 16 + suffix.size()) {
    return false;
  }
  const auto identity = value.substr(prefix.size(), 16);
  return std::ranges::all_of(identity, [](unsigned char byte) {
    return (byte >= '0' && byte <= '9') || (byte >= 'a' && byte <= 'f');
  });
}

std::string file_version(const std::filesystem::path& path) {
  DWORD ignored = 0;
  const auto bytes = GetFileVersionInfoSizeW(path.c_str(), &ignored);
  if (bytes == 0 || bytes > 16U * 1024U * 1024U) {
    fail("The WebView2 runtime version resource is unavailable.");
  }
  std::vector<std::uint8_t> buffer(bytes);
  if (GetFileVersionInfoW(path.c_str(), 0, bytes, buffer.data()) == 0) {
    fail("The WebView2 runtime version resource could not be read.");
  }
  VS_FIXEDFILEINFO* information = nullptr;
  UINT information_bytes = 0;
  if (VerQueryValueW(
          buffer.data(),
          L"\\",
          reinterpret_cast<void**>(&information),
          &information_bytes) == 0 ||
      information == nullptr || information_bytes < sizeof(VS_FIXEDFILEINFO) ||
      information->dwSignature != 0xFEEF04BD) {
    fail("The WebView2 runtime version identity is malformed.");
  }
  std::ostringstream output;
  output << HIWORD(information->dwFileVersionMS) << '.'
         << LOWORD(information->dwFileVersionMS) << '.'
         << HIWORD(information->dwFileVersionLS) << '.'
         << LOWORD(information->dwFileVersionLS);
  return output.str();
}

std::filesystem::path process_image_path(DWORD process_id) {
  HeldHandle process(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, process_id));
  if (!process.valid()) return {};
  std::wstring path(32'768, L'\0');
  DWORD length = static_cast<DWORD>(path.size());
  if (QueryFullProcessImageNameW(process.get(), 0, path.data(), &length) == 0 ||
      length == 0 || length >= path.size()) {
    return {};
  }
  path.resize(length);
  return std::filesystem::path(std::move(path));
}

struct SnapshotProcess final {
  DWORD process_id{};
  DWORD parent_process_id{};
  std::wstring image_name;
};

std::vector<SnapshotProcess> process_snapshot() {
  HeldHandle snapshot(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0));
  if (!snapshot.valid()) fail("The external process snapshot could not be captured.");
  PROCESSENTRY32W entry{};
  entry.dwSize = sizeof(entry);
  std::vector<SnapshotProcess> result;
  if (Process32FirstW(snapshot.get(), &entry) == 0) {
    fail("The external process snapshot was empty.");
  }
  do {
    result.push_back({entry.th32ProcessID, entry.th32ParentProcessID, entry.szExeFile});
  } while (Process32NextW(snapshot.get(), &entry) != 0);
  return result;
}

std::set<DWORD> job_processes(HANDLE process_job) {
  constexpr std::size_t maximum_processes = 4'096;
  std::size_t capacity = 32;
  while (capacity <= maximum_processes) {
    const auto bytes = offsetof(JOBOBJECT_BASIC_PROCESS_ID_LIST, ProcessIdList) +
        capacity * sizeof(ULONG_PTR);
    std::vector<ULONG_PTR> storage(
        (bytes + sizeof(ULONG_PTR) - 1) / sizeof(ULONG_PTR));
    auto* list = reinterpret_cast<JOBOBJECT_BASIC_PROCESS_ID_LIST*>(storage.data());
    SetLastError(ERROR_SUCCESS);
    const BOOL queried = QueryInformationJobObject(
        process_job,
        JobObjectBasicProcessIdList,
        list,
        static_cast<DWORD>(bytes),
        nullptr);
    const DWORD error = queried == 0 ? GetLastError() : ERROR_SUCCESS;
    if (queried == 0 && error != ERROR_MORE_DATA) {
      fail("The scoped process job membership could not be observed.");
    }
    if (list->NumberOfProcessIdsInList < list->NumberOfAssignedProcesses) {
      capacity = std::max<std::size_t>(
          capacity * 2,
          list->NumberOfAssignedProcesses);
      continue;
    }
    std::set<DWORD> result;
    for (DWORD index = 0; index < list->NumberOfProcessIdsInList; ++index) {
      const ULONG_PTR process_id = list->ProcessIdList[index];
      if (process_id == 0 || process_id > MAXDWORD) {
        fail("The scoped process job returned an invalid process identity.");
      }
      result.insert(static_cast<DWORD>(process_id));
    }
    return result;
  }
  fail("The scoped process job membership exceeded its fixed bound.");
}

std::string lowercase_ascii(std::wstring_view value) {
  std::string result;
  result.reserve(value.size());
  for (const auto character : value) {
    if (character > 0x7f) return {};
    result.push_back(static_cast<char>(std::tolower(static_cast<unsigned char>(character))));
  }
  return result;
}

std::string ipv4_text(DWORD address) {
  IN_ADDR value{};
  value.S_un.S_addr = address;
  std::array<char, INET_ADDRSTRLEN> output{};
  if (InetNtopA(AF_INET, &value, output.data(), output.size()) == nullptr) {
    fail("An IPv4 endpoint could not be formatted.");
  }
  return output.data();
}

std::string ipv6_text(const UCHAR* address) {
  IN6_ADDR value{};
  std::copy_n(address, 16, value.u.Byte);
  std::array<char, INET6_ADDRSTRLEN> output{};
  if (InetNtopA(AF_INET6, &value, output.data(), output.size()) == nullptr) {
    fail("An IPv6 endpoint could not be formatted.");
  }
  return output.data();
}

bool external_ipv4(DWORD address, std::uint16_t port) {
  const auto host = ntohl(address);
  return port != 0 && host != 0 && (host >> 24U) != 127U;
}

bool non_loopback_ipv4(DWORD address, bool allow_unspecified) {
  const auto host = ntohl(address);
  return (host != 0 || !allow_unspecified) && (host >> 24U) != 127U;
}

bool external_ipv6(const UCHAR* address, std::uint16_t port) {
  IN6_ADDR value{};
  std::copy_n(address, 16, value.u.Byte);
  return port != 0 && IN6_IS_ADDR_UNSPECIFIED(&value) == 0 &&
      IN6_IS_ADDR_LOOPBACK(&value) == 0;
}

bool non_loopback_ipv6(const UCHAR* address, bool allow_unspecified) {
  IN6_ADDR value{};
  std::copy_n(address, 16, value.u.Byte);
  return IN6_IS_ADDR_LOOPBACK(&value) == 0 &&
      (IN6_IS_ADDR_UNSPECIFIED(&value) == 0 || !allow_unspecified);
}

template <typename Table>
std::vector<std::uint8_t> endpoint_table_bytes(
    ULONG family,
    TCP_TABLE_CLASS table_class) {
  DWORD bytes = 0;
  const auto first = GetExtendedTcpTable(
      nullptr, &bytes, FALSE, family, table_class, 0);
  if (first != ERROR_INSUFFICIENT_BUFFER || bytes == 0 ||
      bytes > 64U * 1024U * 1024U) {
    fail("The external TCP endpoint inventory size failed closed.");
  }
  std::vector<std::uint8_t> result(bytes);
  if (GetExtendedTcpTable(
          result.data(), &bytes, FALSE, family, table_class, 0) != NO_ERROR ||
      bytes < sizeof(Table)) {
    fail("The external TCP endpoint inventory failed closed.");
  }
  result.resize(bytes);
  return result;
}

template <typename Table>
std::vector<std::uint8_t> udp_table_bytes(
    ULONG family,
    UDP_TABLE_CLASS table_class) {
  DWORD bytes = 0;
  const auto first = GetExtendedUdpTable(
      nullptr, &bytes, FALSE, family, table_class, 0);
  if (first != ERROR_INSUFFICIENT_BUFFER || bytes == 0 ||
      bytes > 64U * 1024U * 1024U) {
    fail("The external UDP endpoint inventory size failed closed.");
  }
  std::vector<std::uint8_t> result(bytes);
  if (GetExtendedUdpTable(
          result.data(), &bytes, FALSE, family, table_class, 0) != NO_ERROR ||
      bytes < sizeof(Table)) {
    fail("The external UDP endpoint inventory failed closed.");
  }
  result.resize(bytes);
  return result;
}

void capture_endpoints(
    const std::set<DWORD>& admitted_processes,
    std::map<decltype(Endpoint{}.key()), Endpoint>& observed) {
  {
    const auto bytes = endpoint_table_bytes<MIB_TCPTABLE_OWNER_PID>(
        AF_INET, TCP_TABLE_OWNER_PID_ALL);
    const auto* table = reinterpret_cast<const MIB_TCPTABLE_OWNER_PID*>(bytes.data());
    for (DWORD index = 0; index < table->dwNumEntries; ++index) {
      const auto& row = table->table[index];
      if (!admitted_processes.contains(row.dwOwningPid)) continue;
      Endpoint endpoint{
          row.dwOwningPid,
          "tcp",
          "ipv4",
          ipv4_text(row.dwLocalAddr),
          ntohs(static_cast<u_short>(row.dwLocalPort)),
          ipv4_text(row.dwRemoteAddr),
          ntohs(static_cast<u_short>(row.dwRemotePort)),
          row.dwState,
          false,
      };
      endpoint.external = external_ipv4(row.dwRemoteAddr, endpoint.remote_port) ||
          (row.dwState == MIB_TCP_STATE_LISTEN &&
           non_loopback_ipv4(row.dwLocalAddr, true));
      observed.emplace(endpoint.key(), std::move(endpoint));
    }
  }
  {
    const auto bytes = endpoint_table_bytes<MIB_TCP6TABLE_OWNER_PID>(
        AF_INET6, TCP_TABLE_OWNER_PID_ALL);
    const auto* table = reinterpret_cast<const MIB_TCP6TABLE_OWNER_PID*>(bytes.data());
    for (DWORD index = 0; index < table->dwNumEntries; ++index) {
      const auto& row = table->table[index];
      if (!admitted_processes.contains(row.dwOwningPid)) continue;
      Endpoint endpoint{
          row.dwOwningPid,
          "tcp",
          "ipv6",
          ipv6_text(row.ucLocalAddr),
          ntohs(static_cast<u_short>(row.dwLocalPort)),
          ipv6_text(row.ucRemoteAddr),
          ntohs(static_cast<u_short>(row.dwRemotePort)),
          row.dwState,
          false,
      };
      endpoint.external = external_ipv6(row.ucRemoteAddr, endpoint.remote_port) ||
          (row.dwState == MIB_TCP_STATE_LISTEN &&
           non_loopback_ipv6(row.ucLocalAddr, true));
      observed.emplace(endpoint.key(), std::move(endpoint));
    }
  }
  {
    const auto bytes = udp_table_bytes<MIB_UDPTABLE_OWNER_PID>(
        AF_INET, UDP_TABLE_OWNER_PID);
    const auto* table = reinterpret_cast<const MIB_UDPTABLE_OWNER_PID*>(bytes.data());
    for (DWORD index = 0; index < table->dwNumEntries; ++index) {
      const auto& row = table->table[index];
      if (!admitted_processes.contains(row.dwOwningPid)) continue;
      Endpoint endpoint{
          row.dwOwningPid,
          "udp",
          "ipv4",
          ipv4_text(row.dwLocalAddr),
          ntohs(static_cast<u_short>(row.dwLocalPort)),
          "",
          0,
          0,
          false,
      };
      endpoint.external = non_loopback_ipv4(row.dwLocalAddr, false);
      observed.emplace(endpoint.key(), std::move(endpoint));
    }
  }
  {
    const auto bytes = udp_table_bytes<MIB_UDP6TABLE_OWNER_PID>(
        AF_INET6, UDP_TABLE_OWNER_PID);
    const auto* table = reinterpret_cast<const MIB_UDP6TABLE_OWNER_PID*>(bytes.data());
    for (DWORD index = 0; index < table->dwNumEntries; ++index) {
      const auto& row = table->table[index];
      if (!admitted_processes.contains(row.dwOwningPid)) continue;
      Endpoint endpoint{
          row.dwOwningPid,
          "udp",
          "ipv6",
          ipv6_text(row.ucLocalAddr),
          ntohs(static_cast<u_short>(row.dwLocalPort)),
          "",
          0,
          0,
          false,
      };
      endpoint.external = non_loopback_ipv6(row.ucLocalAddr, false);
      observed.emplace(endpoint.key(), std::move(endpoint));
    }
  }
}

void capture_process_identity(
    DWORD process_id,
    DWORD parent_process_id,
    std::wstring_view image_name,
    const std::filesystem::path& image_path,
    std::uint32_t system_volume_serial,
    ExternalObservation& observation) {
  const auto name = lowercase_ascii(image_name);
  const auto existing = observation.processes.find(process_id);
  std::string digest = existing == observation.processes.end()
      ? std::string{}
      : existing->second.executable_sha256;
  const bool new_runtime_identity =
      name == "msedgewebview2.exe" && !image_path.empty() &&
      (existing == observation.processes.end() ||
       existing->second.executable_sha256.empty());

  if (name == "msedgewebview2.exe" && !image_path.empty()) {
    if (observation.runtime_authorities.empty()) {
      observation.runtime_authorities =
          attest_path_chain(image_path.parent_path(), system_volume_serial);
      observation.runtime_authorities.push_back(
          open_attested_path(
              image_path,
              false,
              system_volume_serial,
              0,
              HardlinkPolicy::allow_multiple));
      observation.runtime_path = image_path;
      observation.runtime_sha256 =
          sha256_handle(observation.runtime_authorities.back().get());
      observation.runtime_version = file_version(image_path);
    } else if (observation.runtime_path.empty() ||
               observation.runtime_sha256.empty() ||
               observation.runtime_version.empty() ||
               normalized_path(observation.runtime_path.wstring()) !=
                   normalized_path(image_path.wstring())) {
      fail("More than one WebView2 runtime identity was observed.");
    }
    digest = narrow_ascii(observation.runtime_sha256);
  } else if (digest.empty() && !image_path.empty()) {
    digest = narrow_ascii(sha256_file(image_path));
  }

  const auto [stored, inserted] = observation.processes.try_emplace(
      process_id,
      ObservedProcess{process_id, parent_process_id, name, digest});
  if (!inserted) {
    if ((!stored->second.image_name.empty() && !name.empty() &&
         stored->second.image_name != name) ||
        (!stored->second.executable_sha256.empty() && !digest.empty() &&
         stored->second.executable_sha256 != digest) ||
        (stored->second.parent_process_id != 0 && parent_process_id != 0 &&
         stored->second.parent_process_id != parent_process_id)) {
      fail("A scoped process identity changed during observation.");
    }
    if (stored->second.parent_process_id == 0 && parent_process_id != 0) {
      stored->second.parent_process_id = parent_process_id;
    }
    if (stored->second.image_name.empty()) stored->second.image_name = name;
    if (stored->second.executable_sha256.empty()) {
      stored->second.executable_sha256 = digest;
    }
  }
  if (name != "msedgewebview2.exe" || image_path.empty()) return;
  if (new_runtime_identity) ++observation.runtime_process_count;
}

void capture_job_notifications(
    HANDLE process_job,
    HANDLE completion_port,
    DWORD initial_wait_milliseconds,
    std::uint32_t system_volume_serial,
    ExternalObservation& observation) {
  DWORD wait_milliseconds = initial_wait_milliseconds;
  while (true) {
    DWORD message = 0;
    ULONG_PTR key = 0;
    OVERLAPPED* value = nullptr;
    if (GetQueuedCompletionStatus(
            completion_port, &message, &key, &value, wait_milliseconds) == 0) {
      if (GetLastError() == WAIT_TIMEOUT) return;
      fail("The scoped process notification stream failed closed.");
    }
    wait_milliseconds = 0;
    if (key != reinterpret_cast<ULONG_PTR>(process_job)) {
      fail("The scoped process notification key changed.");
    }
    if (message != JOB_OBJECT_MSG_NEW_PROCESS || value == nullptr) continue;
    const auto process_id = reinterpret_cast<ULONG_PTR>(value);
    if (process_id == 0 || process_id > MAXDWORD) {
      fail("The scoped process notification returned an invalid identity.");
    }
    const auto image_path = process_image_path(static_cast<DWORD>(process_id));
    if (image_path.empty()) continue;
    capture_process_identity(
        static_cast<DWORD>(process_id),
        0,
        image_path.filename().wstring(),
        image_path,
        system_volume_serial,
        observation);
  }
}

void capture_external_observation(
    HANDLE process_job,
    HANDLE completion_port,
    HANDLE root_handle,
    DWORD root_process,
    DWORD notification_wait_milliseconds,
    std::uint32_t system_volume_serial,
    ExternalObservation& observation) {
  ++observation.snapshot_count;
  capture_job_notifications(
      process_job, completion_port, notification_wait_milliseconds,
      system_volume_serial, observation);
  const auto snapshot = process_snapshot();
  const auto admitted = job_processes(process_job);
  if (!admitted.contains(root_process)) {
    if (WaitForSingleObject(root_handle, 0) == WAIT_OBJECT_0) return;
    fail("The exact staged shell left its scoped process job before exit.");
  }
  capture_endpoints(admitted, observation.endpoints);
  for (const auto& process : snapshot) {
    if (!admitted.contains(process.process_id)) continue;
    const auto image_path = process_image_path(process.process_id);
    capture_process_identity(
        process.process_id,
        process.parent_process_id,
        process.image_name,
        image_path,
        system_volume_serial,
        observation);
  }
}

}  // namespace
