#include "broker_client.h"

#include <bcrypt.h>
#include <shlobj.h>

#include <algorithm>
#include <array>
#include <cwctype>
#include <filesystem>
#include <iomanip>
#include <limits>
#include <memory>
#include <sstream>
#include <span>
#include <string_view>
#include <utility>

#ifndef GOVS_BROKER_SHA256
#error GOVS_BROKER_SHA256 must bind the exact packaged broker before compilation.
#endif
#ifndef GOVS_APP_SHA256
#error GOVS_APP_SHA256 must bind the exact packaged workbench before compilation.
#endif
#ifndef GOVS_PACKAGE_CONTRACT_SHA256
#error GOVS_PACKAGE_CONTRACT_SHA256 must bind the package contract before compilation.
#endif

namespace govs::shell {
namespace {

constexpr std::array<std::uint8_t, 8> kRequestMagic{
    'P', '2', 'V', 'L', 'A', 'B', 'Q', '1'};
constexpr std::array<std::uint8_t, 8> kResponseMagic{
    'P', '2', 'V', 'L', 'A', 'B', 'R', '1'};
constexpr std::uint16_t kProtocolVersion = 1;
constexpr std::size_t kHeaderBytes = 24;
constexpr std::size_t kMaxNameBytes = 255;
constexpr std::size_t kMaxFramePayload = kMaxProjectBytes + kMaxNameBytes + 16;
constexpr DWORD kUnsafeBackingAttributes =
    FILE_ATTRIBUTE_DEVICE | FILE_ATTRIBUTE_REPARSE_POINT |
    FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_VIRTUAL |
    FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_PINNED |
    FILE_ATTRIBUTE_UNPINNED | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS;

constexpr std::uint8_t kHandshake = 0;
constexpr std::uint8_t kList = 1;
constexpr std::uint8_t kOpen = 2;
constexpr std::uint8_t kSaveAs = 3;
constexpr std::uint8_t kSave = 4;
constexpr std::uint8_t kRevoke = 5;

constexpr std::uint8_t kError = 0;
constexpr std::uint8_t kHandshakeResponse = 1;
constexpr std::uint8_t kProjectsResponse = 2;
constexpr std::uint8_t kOpenedResponse = 3;
constexpr std::uint8_t kSavedResponse = 4;
constexpr std::uint8_t kRevokedResponse = 5;

[[noreturn]] void fail(BrokerErrorCode code, const char* message) {
  throw BrokerFailure(code, message);
}

void close_handle(HANDLE& handle) noexcept {
  if (handle != nullptr && handle != INVALID_HANDLE_VALUE) {
    CloseHandle(handle);
  }
  handle = INVALID_HANDLE_VALUE;
}

class HeldHandle final {
 public:
  HeldHandle() = default;
  explicit HeldHandle(HANDLE value) : value_(value) {}
  HeldHandle(const HeldHandle&) = delete;
  HeldHandle& operator=(const HeldHandle&) = delete;
  HeldHandle(HeldHandle&& other) noexcept : value_(std::exchange(other.value_, INVALID_HANDLE_VALUE)) {}
  HeldHandle& operator=(HeldHandle&& other) noexcept {
    if (this != &other) {
      reset();
      value_ = std::exchange(other.value_, INVALID_HANDLE_VALUE);
    }
    return *this;
  }
  ~HeldHandle() { reset(); }
  [[nodiscard]] HANDLE get() const noexcept { return value_; }
  [[nodiscard]] bool valid() const noexcept {
    return value_ != nullptr && value_ != INVALID_HANDLE_VALUE;
  }
  [[nodiscard]] HANDLE release() noexcept {
    return std::exchange(value_, INVALID_HANDLE_VALUE);
  }

 private:
  void reset() noexcept {
    if (valid()) {
      CloseHandle(value_);
    }
    value_ = INVALID_HANDLE_VALUE;
  }
  HANDLE value_{INVALID_HANDLE_VALUE};
};

void write_exact(HANDLE handle, std::span<const std::uint8_t> bytes) {
  while (!bytes.empty()) {
    DWORD written = 0;
    const auto chunk = static_cast<DWORD>(std::min<std::size_t>(
        bytes.size(), std::numeric_limits<DWORD>::max()));
    if (WriteFile(handle, bytes.data(), chunk, &written, nullptr) == 0 ||
        written == 0) {
      fail(BrokerErrorCode::access_unavailable,
           "The private broker input channel failed closed.");
    }
    bytes = bytes.subspan(written);
  }
}

void read_exact(HANDLE handle, std::span<std::uint8_t> bytes) {
  while (!bytes.empty()) {
    DWORD read = 0;
    const auto chunk = static_cast<DWORD>(std::min<std::size_t>(
        bytes.size(), std::numeric_limits<DWORD>::max()));
    if (ReadFile(handle, bytes.data(), chunk, &read, nullptr) == 0 || read == 0) {
      fail(BrokerErrorCode::access_unavailable,
           "The private broker output channel failed closed.");
    }
    bytes = bytes.subspan(read);
  }
}

void append_u16(std::vector<std::uint8_t>& output, std::uint16_t value) {
  output.push_back(static_cast<std::uint8_t>(value));
  output.push_back(static_cast<std::uint8_t>(value >> 8U));
}

void append_u32(std::vector<std::uint8_t>& output, std::uint32_t value) {
  for (unsigned shift = 0; shift < 32; shift += 8) {
    output.push_back(static_cast<std::uint8_t>(value >> shift));
  }
}

void append_u64(std::vector<std::uint8_t>& output, std::uint64_t value) {
  for (unsigned shift = 0; shift < 64; shift += 8) {
    output.push_back(static_cast<std::uint8_t>(value >> shift));
  }
}

void append_name(std::vector<std::uint8_t>& output, const std::string& value) {
  if (value.empty() || value.size() > kMaxNameBytes ||
      !std::ranges::all_of(value, [](unsigned char byte) {
        return byte >= 0x20 && byte <= 0x7e;
      })) {
    fail(BrokerErrorCode::invalid_file_name,
         "The shell rejected an invalid project base name.");
  }
  append_u16(output, static_cast<std::uint16_t>(value.size()));
  output.insert(output.end(), value.begin(), value.end());
}

void append_bytes(
    std::vector<std::uint8_t>& output,
    const std::vector<std::uint8_t>& bytes) {
  if (bytes.empty() || bytes.size() > kMaxProjectBytes) {
    fail(BrokerErrorCode::project_too_large,
         "The shell rejected an out-of-bounds project payload.");
  }
  append_u32(output, static_cast<std::uint32_t>(bytes.size()));
  output.insert(output.end(), bytes.begin(), bytes.end());
}

class Cursor final {
 public:
  explicit Cursor(const std::vector<std::uint8_t>& bytes) : bytes_(bytes) {}

  [[nodiscard]] std::uint8_t u8() {
    require(1);
    return bytes_[offset_++];
  }

  [[nodiscard]] std::uint16_t u16() {
    require(2);
    const auto value = static_cast<std::uint16_t>(
        static_cast<std::uint16_t>(bytes_[offset_]) |
        static_cast<std::uint16_t>(
            static_cast<std::uint16_t>(bytes_[offset_ + 1]) << 8U));
    offset_ += 2;
    return value;
  }

  [[nodiscard]] std::uint32_t u32() {
    require(4);
    std::uint32_t value = 0;
    for (unsigned index = 0; index < 4; ++index) {
      value |= static_cast<std::uint32_t>(bytes_[offset_ + index]) << (index * 8U);
    }
    offset_ += 4;
    return value;
  }

  [[nodiscard]] std::uint64_t u64() {
    require(8);
    std::uint64_t value = 0;
    for (unsigned index = 0; index < 8; ++index) {
      value |= static_cast<std::uint64_t>(bytes_[offset_ + index]) << (index * 8U);
    }
    offset_ += 8;
    return value;
  }

  [[nodiscard]] std::string name() {
    const auto length = static_cast<std::size_t>(u16());
    if (length == 0 || length > kMaxNameBytes) {
      invalid();
    }
    require(length);
    std::string value(
        reinterpret_cast<const char*>(bytes_.data() + offset_), length);
    offset_ += length;
    if (!std::ranges::all_of(value, [](unsigned char byte) {
          return byte >= 0x20 && byte <= 0x7e;
        })) {
      invalid();
    }
    return value;
  }

  [[nodiscard]] std::vector<std::uint8_t> bytes() {
    const auto length = static_cast<std::size_t>(u32());
    if (length == 0 || length > kMaxProjectBytes) {
      invalid();
    }
    require(length);
    std::vector<std::uint8_t> value(
        bytes_.begin() + static_cast<std::ptrdiff_t>(offset_),
        bytes_.begin() + static_cast<std::ptrdiff_t>(offset_ + length));
    offset_ += length;
    return value;
  }

  void finish() const {
    if (offset_ != bytes_.size()) {
      invalid();
    }
  }

 private:
  void require(std::size_t length) const {
    if (length > bytes_.size() - std::min(offset_, bytes_.size())) {
      invalid();
    }
  }

  [[noreturn]] static void invalid() {
    fail(BrokerErrorCode::invalid_frame,
         "The shell rejected a malformed broker response.");
  }

  const std::vector<std::uint8_t>& bytes_;
  std::size_t offset_{};
};

std::filesystem::path executable_path() {
  std::wstring buffer(32'768, L'\0');
  const auto length = GetModuleFileNameW(
      nullptr, buffer.data(), static_cast<DWORD>(buffer.size()));
  if (length == 0 || length >= buffer.size()) {
    fail(BrokerErrorCode::access_unavailable,
         "The Windows shell executable location is unavailable.");
  }
  buffer.resize(length);
  return std::filesystem::path(buffer);
}

std::filesystem::path executable_directory() {
  return executable_path().parent_path();
}

std::wstring normalized_path(std::wstring value) {
  if (value.starts_with(LR"(\\?\)")) {
    value.erase(0, 4);
  }
  std::ranges::replace(value, L'/', L'\\');
  while (value.size() > 3 && value.back() == L'\\') {
    value.pop_back();
  }
  std::ranges::transform(value, value.begin(), [](wchar_t character) {
    return static_cast<wchar_t>(std::towlower(character));
  });
  return value;
}

std::wstring final_path(HANDLE handle) {
  const auto required = GetFinalPathNameByHandleW(
      handle, nullptr, 0, FILE_NAME_NORMALIZED | VOLUME_NAME_DOS);
  if (required == 0 || required > 32'767) {
    fail(BrokerErrorCode::attestation_failed,
         "The package authority returned no bounded final DOS path.");
  }
  std::wstring buffer(static_cast<std::size_t>(required) + 1U, L'\0');
  const auto written = GetFinalPathNameByHandleW(
      handle,
      buffer.data(),
      static_cast<DWORD>(buffer.size()),
      FILE_NAME_NORMALIZED | VOLUME_NAME_DOS);
  if (written == 0 || written >= buffer.size()) {
    fail(BrokerErrorCode::attestation_failed,
         "The package authority final DOS path failed closed.");
  }
  buffer.resize(written);
  return normalized_path(std::move(buffer));
}

std::uint32_t fixed_volume_serial(const std::filesystem::path& path) {
  const auto root = path.root_path().wstring();
  if (root.size() != 3 || GetDriveTypeW(root.c_str()) != DRIVE_FIXED) {
    fail(BrokerErrorCode::attestation_failed,
         "The Windows shell package is not on a fixed local drive.");
  }
  std::array<wchar_t, 32'768> windows_directory{};
  const auto windows_length = GetWindowsDirectoryW(
      windows_directory.data(), static_cast<UINT>(windows_directory.size()));
  if (windows_length == 0 || windows_length >= windows_directory.size() ||
      normalized_path(std::filesystem::path(
                          std::wstring_view(windows_directory.data(), windows_length))
                          .root_path()
                          .wstring()) != normalized_path(root)) {
    fail(BrokerErrorCode::attestation_failed,
         "The Windows shell package is outside the native system volume.");
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
  if (!volume.valid()) {
    fail(BrokerErrorCode::attestation_failed,
         "The native system storage device could not be opened.");
  }
  STORAGE_PROPERTY_QUERY query{};
  query.PropertyId = StorageDeviceProperty;
  query.QueryType = PropertyStandardQuery;
  alignas(STORAGE_DEVICE_DESCRIPTOR) std::array<std::uint8_t, 1'024>
      descriptor_buffer{};
  DWORD returned = 0;
  if (DeviceIoControl(
          volume.get(),
          IOCTL_STORAGE_QUERY_PROPERTY,
          &query,
          sizeof(query),
          descriptor_buffer.data(),
          static_cast<DWORD>(descriptor_buffer.size()),
          &returned,
          nullptr) == 0 ||
      returned < offsetof(STORAGE_DEVICE_DESCRIPTOR, RawDeviceProperties)) {
    fail(BrokerErrorCode::attestation_failed,
         "The native system storage descriptor failed closed.");
  }
  STORAGE_HOTPLUG_INFO hotplug{};
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
    fail(BrokerErrorCode::attestation_failed,
         "The native system storage hotplug status failed closed.");
  }
  const auto& descriptor = *reinterpret_cast<const STORAGE_DEVICE_DESCRIPTOR*>(
      descriptor_buffer.data());
  const bool admitted_bus =
      descriptor.BusType == BusTypeScsi || descriptor.BusType == BusTypeAtapi ||
      descriptor.BusType == BusTypeAta || descriptor.BusType == BusTypeRAID ||
      descriptor.BusType == BusTypeSas || descriptor.BusType == BusTypeSata ||
      descriptor.BusType == BusTypeNvme;
  if (!admitted_bus || descriptor.RemovableMedia != FALSE ||
      hotplug.MediaRemovable != FALSE || hotplug.MediaHotplug != FALSE ||
      hotplug.DeviceHotplug != FALSE) {
    fail(BrokerErrorCode::attestation_failed,
         "The Windows shell package storage is removable, virtual, remote, hotplug, or non-native.");
  }
  std::uint32_t serial = 0;
  std::array<wchar_t, 32> file_system{};
  if (GetVolumeInformationW(
          root.c_str(),
          nullptr,
          0,
          reinterpret_cast<LPDWORD>(&serial),
          nullptr,
          nullptr,
          file_system.data(),
          static_cast<DWORD>(file_system.size())) == 0) {
    fail(BrokerErrorCode::attestation_failed,
         "The Windows shell package volume could not be attested.");
  }
  const std::wstring_view type(file_system.data());
  if (type != L"NTFS" && type != L"ReFS") {
    fail(BrokerErrorCode::attestation_failed,
         "The Windows shell package filesystem is not admitted.");
  }
  return serial;
}

HeldHandle open_attested_path(
    const std::filesystem::path& path,
    bool directory,
    std::uint32_t expected_serial) {
  const auto handle = CreateFileW(
      path.c_str(),
      directory ? 0U : GENERIC_READ,
      FILE_SHARE_READ,
      nullptr,
      OPEN_EXISTING,
      FILE_FLAG_OPEN_REPARSE_POINT |
          (directory ? FILE_FLAG_BACKUP_SEMANTICS : FILE_FLAG_SEQUENTIAL_SCAN),
      nullptr);
  HeldHandle authority(handle);
  if (!authority.valid()) {
    fail(BrokerErrorCode::attestation_failed,
         "The Windows shell package authority could not be opened.");
  }
  BY_HANDLE_FILE_INFORMATION information{};
  if (GetFileInformationByHandle(authority.get(), &information) == 0 ||
      information.dwVolumeSerialNumber != expected_serial ||
      (information.dwFileAttributes & kUnsafeBackingAttributes) != 0 ||
      ((information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0) != directory ||
      (!directory && information.nNumberOfLinks != 1) ||
      final_path(authority.get()) != normalized_path(path.wstring())) {
    fail(BrokerErrorCode::attestation_failed,
         "The Windows shell package authority failed fixed-local attestation.");
  }
  FILE_REMOTE_PROTOCOL_INFO remote{};
  if (GetFileInformationByHandleEx(
          authority.get(), FileRemoteProtocolInfo, &remote, sizeof(remote)) != 0) {
    fail(BrokerErrorCode::attestation_failed,
         "The Windows shell package authority is remote.");
  }
  const auto remote_error = GetLastError();
  if (remote_error != ERROR_INVALID_FUNCTION &&
      remote_error != ERROR_NOT_SUPPORTED &&
      remote_error != ERROR_INVALID_PARAMETER) {
    fail(BrokerErrorCode::attestation_failed,
         "The Windows shell package remote status was inconclusive.");
  }
  return authority;
}

std::wstring sha256(HANDLE file) {
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
    fail(BrokerErrorCode::attestation_failed,
         "The package SHA-256 provider failed closed.");
  }
  std::vector<std::uint8_t> object(object_bytes);
  if (BCryptCreateHash(
          algorithm, &hash, object.data(), object_bytes, nullptr, 0, 0) < 0) {
    BCryptCloseAlgorithmProvider(algorithm, 0);
    fail(BrokerErrorCode::attestation_failed,
         "The package SHA-256 state failed closed.");
  }
  LARGE_INTEGER start{};
  if (SetFilePointerEx(file, start, nullptr, FILE_BEGIN) == 0) {
    BCryptDestroyHash(hash);
    BCryptCloseAlgorithmProvider(algorithm, 0);
    fail(BrokerErrorCode::attestation_failed,
         "The fixed broker hash input failed closed.");
  }
  std::array<std::uint8_t, 64 * 1024> buffer{};
  while (true) {
    DWORD read = 0;
    if (ReadFile(file, buffer.data(), static_cast<DWORD>(buffer.size()), &read, nullptr) == 0) {
      BCryptDestroyHash(hash);
      BCryptCloseAlgorithmProvider(algorithm, 0);
      fail(BrokerErrorCode::attestation_failed,
           "The fixed broker hash read failed closed.");
    }
    if (read == 0) break;
    if (BCryptHashData(hash, buffer.data(), read, 0) < 0) {
      BCryptDestroyHash(hash);
      BCryptCloseAlgorithmProvider(algorithm, 0);
      fail(BrokerErrorCode::attestation_failed,
           "The fixed broker hash update failed closed.");
    }
  }
  std::array<std::uint8_t, 32> digest{};
  if (BCryptFinishHash(hash, digest.data(), static_cast<ULONG>(digest.size()), 0) < 0) {
    BCryptDestroyHash(hash);
    BCryptCloseAlgorithmProvider(algorithm, 0);
    fail(BrokerErrorCode::attestation_failed,
         "The fixed broker hash finalization failed closed.");
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

struct PackageAuthority final {
  std::vector<HeldHandle> directories;
  HeldHandle shell;
  HeldHandle broker;
  HeldHandle application;
  HeldHandle contract;
};

PackageAuthority attest_fixed_package(
    const std::filesystem::path& directory,
    const std::filesystem::path& broker_path) {
  const auto serial = fixed_volume_serial(directory);
  PackageAuthority authority{};
  auto current = directory.root_path();
  authority.directories.push_back(open_attested_path(current, true, serial));
  for (const auto& component : directory.relative_path()) {
    current /= component;
    authority.directories.push_back(open_attested_path(current, true, serial));
  }
  const auto shell_path = executable_path();
  if (normalized_path(shell_path.parent_path().wstring()) !=
      normalized_path(directory.wstring())) {
    fail(BrokerErrorCode::attestation_failed,
         "The running shell is outside its attested package directory.");
  }
  authority.shell = open_attested_path(shell_path, false, serial);
  authority.broker = open_attested_path(broker_path, false, serial);
  const std::wstring expected_broker = GOVS_BROKER_SHA256;
  if (expected_broker.size() != 64 ||
      sha256(authority.broker.get()) != expected_broker) {
    fail(BrokerErrorCode::attestation_failed,
         "The fixed broker executable does not match the packaged SHA-256 binding.");
  }
  const auto application_directory = directory / L"app";
  authority.directories.push_back(
      open_attested_path(application_directory, true, serial));
  authority.application = open_attested_path(
      application_directory / L"index.html", false, serial);
  const std::wstring expected_application = GOVS_APP_SHA256;
  if (expected_application.size() != 64 ||
      sha256(authority.application.get()) != expected_application) {
    fail(BrokerErrorCode::attestation_failed,
         "The packaged workbench does not match its SHA-256 binding.");
  }
  authority.contract = open_attested_path(
      directory / L"package-contract-v1.json", false, serial);
  const std::wstring expected_contract = GOVS_PACKAGE_CONTRACT_SHA256;
  if (expected_contract.size() != 64 ||
      sha256(authority.contract.get()) != expected_contract) {
    fail(BrokerErrorCode::attestation_failed,
         "The package contract does not match its SHA-256 binding.");
  }
  return authority;
}

std::filesystem::path authoritative_known_folder(const KNOWNFOLDERID& folder) {
  PWSTR raw = nullptr;
  if (FAILED(SHGetKnownFolderPath(folder, 0, nullptr, &raw)) ||
      raw == nullptr) {
    fail(BrokerErrorCode::attestation_failed,
         "The authoritative Windows known folder is unavailable.");
  }
  const std::filesystem::path path(raw);
  CoTaskMemFree(raw);
  if (!path.is_absolute() || path.has_root_name() == false ||
      path.root_path().wstring().size() != 3) {
    fail(BrokerErrorCode::attestation_failed,
         "The authoritative LocalAppData path was not a bounded drive path.");
  }
  return path;
}

std::filesystem::path authoritative_local_app_data() {
  const auto local = authoritative_known_folder(FOLDERID_LocalAppData);
  const auto profile = authoritative_known_folder(FOLDERID_Profile);
  if (normalized_path(local.wstring()) !=
      normalized_path((profile / L"AppData" / L"Local").wstring())) {
    fail(BrokerErrorCode::attestation_failed,
         "The authoritative LocalAppData folder is redirected.");
  }
  return local;
}

}  // namespace

struct BrokerClient::Response final {
  std::uint8_t tag{};
  std::vector<std::uint8_t> payload;
};

BrokerFailure::BrokerFailure(BrokerErrorCode code, const char* message)
    : std::runtime_error(message), code_(code) {}

BrokerErrorCode BrokerFailure::code() const noexcept { return code_; }

BrokerClient::~BrokerClient() { stop(); }

BrokerAttestation BrokerClient::start() {
  if (process_ != INVALID_HANDLE_VALUE) {
    fail(BrokerErrorCode::access_unavailable,
         "The fixed native broker was already started.");
  }

  const auto directory = executable_directory();
  const auto broker_path = directory / L"windows-project-broker.exe";
  auto package_authority = attest_fixed_package(directory, broker_path);

  SECURITY_ATTRIBUTES security{};
  security.nLength = sizeof(security);
  security.bInheritHandle = TRUE;

  HANDLE child_input = INVALID_HANDLE_VALUE;
  HANDLE parent_input = INVALID_HANDLE_VALUE;
  HANDLE parent_output = INVALID_HANDLE_VALUE;
  HANDLE child_output = INVALID_HANDLE_VALUE;
  if (CreatePipe(&child_input, &parent_input, &security, 0) == 0 ||
      CreatePipe(&parent_output, &child_output, &security, 0) == 0 ||
      SetHandleInformation(parent_input, HANDLE_FLAG_INHERIT, 0) == 0 ||
      SetHandleInformation(parent_output, HANDLE_FLAG_INHERIT, 0) == 0) {
    close_handle(child_input);
    close_handle(parent_input);
    close_handle(parent_output);
    close_handle(child_output);
    fail(BrokerErrorCode::access_unavailable,
         "The shell could not create the private broker pipes.");
  }

  SIZE_T attribute_bytes = 0;
  InitializeProcThreadAttributeList(nullptr, 1, 0, &attribute_bytes);
  auto attributes = std::make_unique<std::uint8_t[]>(attribute_bytes);
  auto* attribute_list =
      reinterpret_cast<LPPROC_THREAD_ATTRIBUTE_LIST>(attributes.get());
  HANDLE admitted_handles[]{child_input, child_output};
  if (InitializeProcThreadAttributeList(
          attribute_list, 1, 0, &attribute_bytes) == 0 ||
      UpdateProcThreadAttribute(
          attribute_list,
          0,
          PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
          admitted_handles,
          sizeof(admitted_handles),
          nullptr,
          nullptr) == 0) {
    close_handle(child_input);
    close_handle(parent_input);
    close_handle(parent_output);
    close_handle(child_output);
    fail(BrokerErrorCode::access_unavailable,
         "The shell could not restrict broker handle inheritance.");
  }

  STARTUPINFOEXW startup{};
  startup.StartupInfo.cb = sizeof(startup);
  startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
  startup.StartupInfo.hStdInput = child_input;
  startup.StartupInfo.hStdOutput = child_output;
  startup.StartupInfo.hStdError = child_output;
  startup.lpAttributeList = attribute_list;

  JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits{};
  limits.BasicLimitInformation.LimitFlags =
      JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
  limits.BasicLimitInformation.ActiveProcessLimit = 1;
  HANDLE job = CreateJobObjectW(nullptr, nullptr);
  if (job == nullptr ||
      SetInformationJobObject(
          job,
          JobObjectExtendedLimitInformation,
          &limits,
          sizeof(limits)) == 0) {
    if (job != nullptr) CloseHandle(job);
    DeleteProcThreadAttributeList(attribute_list);
    close_handle(child_input);
    close_handle(child_output);
    close_handle(parent_input);
    close_handle(parent_output);
    fail(BrokerErrorCode::access_unavailable,
         "The shell could not establish one-process broker containment.");
  }

  std::wstring command_line = L"\"" + broker_path.wstring() + L"\"";
  PROCESS_INFORMATION process{};
  const auto created = CreateProcessW(
      broker_path.c_str(),
      command_line.data(),
      nullptr,
      nullptr,
      TRUE,
      CREATE_NO_WINDOW | CREATE_SUSPENDED | EXTENDED_STARTUPINFO_PRESENT,
      nullptr,
      directory.c_str(),
      &startup.StartupInfo,
      &process);
  DeleteProcThreadAttributeList(attribute_list);
  close_handle(child_input);
  close_handle(child_output);
  if (created == 0) {
    CloseHandle(job);
    close_handle(parent_input);
    close_handle(parent_output);
    fail(BrokerErrorCode::access_unavailable,
         "The shell could not start its fixed native broker.");
  }
  if (AssignProcessToJobObject(job, process.hProcess) == 0 ||
      ResumeThread(process.hThread) == static_cast<DWORD>(-1)) {
    CloseHandle(job);
    TerminateProcess(process.hProcess, 1);
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    close_handle(parent_input);
    close_handle(parent_output);
    fail(BrokerErrorCode::access_unavailable,
         "The shell could not contain its fixed native broker.");
  }
  CloseHandle(process.hThread);

  input_ = parent_input;
  output_ = parent_output;
  process_ = process.hProcess;
  job_ = job;

  const auto response = transact(kHandshake, {});
  if (response.tag != kHandshakeResponse) {
    stop();
    fail(BrokerErrorCode::invalid_frame,
         "The fixed native broker returned no attestation.");
  }
  Cursor cursor(response.payload);
  BrokerAttestation attestation{};
  attestation.protocol_version = cursor.u16();
  attestation.file_system = cursor.u8();
  attestation.volume_serial = cursor.u64();
  attestation.fixed_drive = cursor.u8() == 1;
  attestation.native_local = cursor.u8() == 1;
  attestation.provider_backed = cursor.u8() == 1;
  attestation.redirected = cursor.u8() == 1;
  attestation.removable = cursor.u8() == 1;
  attestation.special = cursor.u8() == 1;
  cursor.finish();
  if (attestation.protocol_version != kProtocolVersion ||
      (attestation.file_system != 1 && attestation.file_system != 2) ||
      !attestation.fixed_drive || !attestation.native_local ||
      attestation.provider_backed || attestation.redirected ||
      attestation.removable || attestation.special) {
    stop();
    fail(BrokerErrorCode::attestation_failed,
         "The fixed native broker attestation failed closed.");
  }
  for (auto& directory_handle : package_authority.directories) {
    authority_handles_.push_back(directory_handle.release());
  }
  authority_handles_.push_back(package_authority.shell.release());
  authority_handles_.push_back(package_authority.broker.release());
  authority_handles_.push_back(package_authority.application.release());
  authority_handles_.push_back(package_authority.contract.release());
  return attestation;
}

std::filesystem::path BrokerClient::prepare_fixed_user_data_folder(
    std::uint64_t session_id) {
  if (process_ == INVALID_HANDLE_VALUE || authority_handles_.empty()) {
    fail(BrokerErrorCode::access_unavailable,
         "The broker must attest the package before preparing browser state.");
  }
  const auto local = authoritative_local_app_data();
  const auto serial = fixed_volume_serial(local);
  HeldHandle local_authority = open_attested_path(local, true, serial);
  const auto application = local / L"GovsPLC";
  if (CreateDirectoryW(application.c_str(), nullptr) == 0 &&
      GetLastError() != ERROR_ALREADY_EXISTS) {
    fail(BrokerErrorCode::attestation_failed,
         "The fixed application state directory could not be created.");
  }
  HeldHandle application_authority =
      open_attested_path(application, true, serial);
  if (session_id == 0) {
    fail(BrokerErrorCode::attestation_failed,
         "The fixed WebView2 session identity is invalid.");
  }
  std::wostringstream session_name;
  session_name << L"WebView2-" << std::hex << std::setw(16)
               << std::setfill(L'0') << session_id;
  const auto user_data = application / session_name.str();
  if (CreateDirectoryW(user_data.c_str(), nullptr) == 0) {
    fail(BrokerErrorCode::attestation_failed,
         "The fresh fixed WebView2 state directory could not be created.");
  }
  HeldHandle user_data_authority = open_attested_path(user_data, true, serial);
  authority_handles_.push_back(local_authority.release());
  authority_handles_.push_back(application_authority.release());
  authority_handles_.push_back(user_data_authority.release());
  return user_data;
}

void BrokerClient::stop() noexcept {
  close_handle(input_);
  close_handle(output_);
  if (process_ != INVALID_HANDLE_VALUE) {
    if (WaitForSingleObject(process_, 2'000) == WAIT_TIMEOUT &&
        job_ != INVALID_HANDLE_VALUE) {
      TerminateJobObject(job_, 1);
      WaitForSingleObject(process_, 2'000);
    }
    close_handle(process_);
  }
  close_handle(job_);
  for (auto& handle : authority_handles_) {
    close_handle(handle);
  }
  authority_handles_.clear();
  next_request_id_ = 1;
}

std::vector<std::string> BrokerClient::list_projects() {
  const auto response = transact(kList, {});
  if (response.tag != kProjectsResponse) {
    fail(BrokerErrorCode::invalid_frame,
         "The broker returned an unexpected project-list response.");
  }
  Cursor cursor(response.payload);
  const auto count = cursor.u16();
  if (count > 4096) {
    fail(BrokerErrorCode::invalid_frame,
         "The broker returned too many project choices.");
  }
  std::vector<std::string> names;
  names.reserve(count);
  for (std::uint16_t index = 0; index < count; ++index) {
    names.push_back(cursor.name());
  }
  cursor.finish();
  return names;
}

OpenedProject BrokerClient::open(const std::string& name) {
  std::vector<std::uint8_t> payload;
  append_name(payload, name);
  const auto response = transact(kOpen, payload);
  if (response.tag != kOpenedResponse) {
    fail(BrokerErrorCode::invalid_frame,
         "The broker returned an unexpected open response.");
  }
  Cursor cursor(response.payload);
  OpenedProject result{cursor.name(), cursor.u64(), cursor.bytes()};
  cursor.finish();
  return result;
}

SavedProject BrokerClient::save_as(
    const std::string& name,
    const std::vector<std::uint8_t>& bytes) {
  std::vector<std::uint8_t> payload;
  append_name(payload, name);
  append_bytes(payload, bytes);
  const auto response = transact(kSaveAs, payload);
  if (response.tag != kSavedResponse) {
    fail(BrokerErrorCode::invalid_frame,
         "The broker returned an unexpected save-as response.");
  }
  Cursor cursor(response.payload);
  SavedProject result{cursor.name(), cursor.u64(), cursor.u64()};
  cursor.finish();
  return result;
}

SavedProject BrokerClient::save(
    std::uint64_t grant_id,
    const std::vector<std::uint8_t>& bytes) {
  std::vector<std::uint8_t> payload;
  append_u64(payload, grant_id);
  append_bytes(payload, bytes);
  const auto response = transact(kSave, payload);
  if (response.tag != kSavedResponse) {
    fail(BrokerErrorCode::invalid_frame,
         "The broker returned an unexpected save response.");
  }
  Cursor cursor(response.payload);
  SavedProject result{cursor.name(), cursor.u64(), cursor.u64()};
  cursor.finish();
  return result;
}

void BrokerClient::revoke(std::uint64_t grant_id) {
  std::vector<std::uint8_t> payload;
  append_u64(payload, grant_id);
  const auto response = transact(kRevoke, payload);
  if (response.tag != kRevokedResponse || !response.payload.empty()) {
    fail(BrokerErrorCode::invalid_frame,
         "The broker returned an unexpected revoke response.");
  }
}

BrokerClient::Response BrokerClient::transact(
    std::uint8_t operation,
    const std::vector<std::uint8_t>& payload) {
  if (input_ == INVALID_HANDLE_VALUE || output_ == INVALID_HANDLE_VALUE ||
      payload.size() > kMaxFramePayload || next_request_id_ == 0) {
    fail(BrokerErrorCode::access_unavailable,
         "The private broker channel is unavailable.");
  }
  const auto request_id = next_request_id_;
  if (next_request_id_ == std::numeric_limits<std::uint64_t>::max()) {
    fail(BrokerErrorCode::access_unavailable,
         "The private broker request identity space was exhausted.");
  }
  ++next_request_id_;

  std::vector<std::uint8_t> frame;
  frame.reserve(kHeaderBytes + payload.size());
  frame.insert(frame.end(), kRequestMagic.begin(), kRequestMagic.end());
  append_u16(frame, kProtocolVersion);
  frame.push_back(operation);
  frame.push_back(0);
  append_u64(frame, request_id);
  append_u32(frame, static_cast<std::uint32_t>(payload.size()));
  frame.insert(frame.end(), payload.begin(), payload.end());
  write_exact(input_, frame);

  std::array<std::uint8_t, kHeaderBytes> header{};
  read_exact(output_, header);
  if (!std::equal(kResponseMagic.begin(), kResponseMagic.end(), header.begin()) ||
      header[8] != 1 || header[9] != 0 || header[11] != 0) {
    fail(BrokerErrorCode::invalid_frame,
         "The broker response header failed closed.");
  }
  std::uint64_t response_id = 0;
  for (unsigned index = 0; index < 8; ++index) {
    response_id |= static_cast<std::uint64_t>(header[12 + index]) << (index * 8U);
  }
  std::uint32_t payload_length = 0;
  for (unsigned index = 0; index < 4; ++index) {
    payload_length |= static_cast<std::uint32_t>(header[20 + index]) << (index * 8U);
  }
  if (response_id != request_id || payload_length > kMaxFramePayload) {
    fail(BrokerErrorCode::invalid_frame,
         "The broker response identity or bound failed closed.");
  }
  Response response{header[10], std::vector<std::uint8_t>(payload_length)};
  read_exact(output_, response.payload);
  if (response.tag == kError) {
    Cursor cursor(response.payload);
    const auto raw_code = cursor.u16();
    const auto message_length = cursor.u16();
    for (std::uint16_t index = 0; index < message_length; ++index) {
      const auto byte = cursor.u8();
      if (byte < 0x20 || byte > 0x7e) {
        fail(BrokerErrorCode::invalid_frame,
             "The broker returned malformed error metadata.");
      }
    }
    cursor.finish();
    if (raw_code < 1 || raw_code > 11) {
      fail(BrokerErrorCode::invalid_frame,
           "The broker returned an unknown error code.");
    }
    throw BrokerFailure(
        static_cast<BrokerErrorCode>(raw_code),
        "The fixed native project broker rejected the request.");
  }
  return response;
}

}  // namespace govs::shell
