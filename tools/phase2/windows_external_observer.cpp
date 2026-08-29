#define WIN32_LEAN_AND_MEAN
#ifndef NOMINMAX
#define NOMINMAX
#endif

#include <windows.h>
#include <bcrypt.h>
#include <evntrace.h>
#include <evntcons.h>
#include <tdh.h>

#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <map>
#include <mutex>
#include <optional>
#include <set>
#include <sstream>
#include <stdexcept>
#include <string>
#include <string_view>
#include <thread>
#include <utility>
#include <vector>

#include "external_observer_candidate.h"

namespace {

constexpr wchar_t kObserverVersion[] = L"govs-p2-windows-etw-observer-v1";
constexpr wchar_t kSessionName[] = L"GovsPLC-Phase2-External-Observer-v1";
constexpr GUID kSessionGuid{
    0x20c18b26, 0x8c38, 0x4cc8, {0xbb, 0x04, 0x5d, 0x68, 0x32, 0xfc, 0x0f, 0x01}};
constexpr GUID kDnsClient{
    0x1c95126e, 0x7eea, 0x49a9, {0xa3, 0xfe, 0xa3, 0x78, 0xb0, 0x3d, 0xdb, 0x4d}};
constexpr GUID kKernelProcess{
    0x22fb2cd6, 0x0e7b, 0x422b, {0xa0, 0xc7, 0x2f, 0xad, 0x1f, 0xd0, 0xe7, 0x16}};
constexpr GUID kNameResolution{
    0x55404e71, 0x4db9, 0x4deb, {0xa5, 0xf5, 0x8f, 0x86, 0xe4, 0x6d, 0xde, 0x56}};
constexpr GUID kKernelNetwork{
    0x7dd42a49, 0x5329, 0x4832, {0x8d, 0xfd, 0x43, 0xd9, 0x79, 0x15, 0x3a, 0x88}};
constexpr GUID kWinsockAfd{
    0xe53c6823, 0x7bb8, 0x44bb, {0x90, 0xdc, 0x3f, 0x86, 0x09, 0x0d, 0x48, 0xa6}};

struct ProviderDefinition {
  const GUID* id;
  const wchar_t* role;
  ULONGLONG match_any_keyword;
};

constexpr std::array<ProviderDefinition, 5> kProviders{{
    {&kDnsClient, L"dns-client", ~0ULL},
    {&kKernelProcess, L"process-ancestry", 0x50ULL},
    {&kNameResolution, L"resolver-api", ~0ULL},
    {&kKernelNetwork, L"packet", ~0ULL},
    {&kWinsockAfd, L"endpoint-socket", ~0ULL},
}};

struct UniqueHandle {
  HANDLE value{INVALID_HANDLE_VALUE};
  UniqueHandle() = default;
  explicit UniqueHandle(HANDLE input) : value(input) {}
  UniqueHandle(const UniqueHandle&) = delete;
  UniqueHandle& operator=(const UniqueHandle&) = delete;
  UniqueHandle(UniqueHandle&& other) noexcept : value(std::exchange(other.value, INVALID_HANDLE_VALUE)) {}
  UniqueHandle& operator=(UniqueHandle&& other) noexcept {
    if (this != &other) {
      reset();
      value = std::exchange(other.value, INVALID_HANDLE_VALUE);
    }
    return *this;
  }
  ~UniqueHandle() { reset(); }
  [[nodiscard]] bool valid() const { return value != nullptr && value != INVALID_HANDLE_VALUE; }
  void reset() {
    if (valid()) CloseHandle(value);
    value = INVALID_HANDLE_VALUE;
  }
};

[[noreturn]] void fail(std::string_view message) { throw std::runtime_error(std::string(message)); }

std::string utf8(std::wstring_view value) {
  if (value.empty()) return {};
  const int size = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                                      static_cast<int>(value.size()), nullptr, 0, nullptr, nullptr);
  if (size <= 0) fail("A Windows string could not be encoded as UTF-8.");
  std::string output(static_cast<std::size_t>(size), '\0');
  if (WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                          static_cast<int>(value.size()), output.data(), size, nullptr, nullptr) != size) {
    fail("A Windows string changed while encoding as UTF-8.");
  }
  return output;
}

std::wstring lower(std::wstring value) {
  std::transform(value.begin(), value.end(), value.begin(), [](wchar_t character) {
    return static_cast<wchar_t>(towlower(character));
  });
  return value;
}

std::string json_escape(std::string_view value) {
  std::ostringstream output;
  for (const unsigned char character : value) {
    switch (character) {
      case '"': output << "\\\""; break;
      case '\\': output << "\\\\"; break;
      case '\b': output << "\\b"; break;
      case '\f': output << "\\f"; break;
      case '\n': output << "\\n"; break;
      case '\r': output << "\\r"; break;
      case '\t': output << "\\t"; break;
      default:
        if (character < 0x20U) {
          output << "\\u00" << std::uppercase << std::hex << std::setw(2)
                 << std::setfill('0') << static_cast<unsigned>(character) << std::dec;
        } else {
          output << static_cast<char>(character);
        }
    }
  }
  return output.str();
}

std::wstring guid_text(const GUID& guid) {
  std::wostringstream output;
  output << std::uppercase << std::hex << std::setfill(L'0')
         << std::setw(8) << guid.Data1 << L'-' << std::setw(4) << guid.Data2 << L'-'
         << std::setw(4) << guid.Data3 << L'-' << std::setw(2)
         << static_cast<unsigned>(guid.Data4[0]) << std::setw(2)
         << static_cast<unsigned>(guid.Data4[1]) << L'-';
  for (std::size_t index = 2; index < 8; ++index) {
    output << std::setw(2) << static_cast<unsigned>(guid.Data4[index]);
  }
  return output.str();
}

std::string hex_u64(ULONGLONG value) {
  std::ostringstream output;
  output << "0x" << std::uppercase << std::hex << std::setfill('0') << std::setw(16) << value;
  return output.str();
}

std::string hex_bytes(const BYTE* bytes, std::size_t size) {
  std::ostringstream output;
  output << std::uppercase << std::hex << std::setfill('0');
  for (std::size_t index = 0; index < size; ++index) {
    output << std::setw(2) << static_cast<unsigned>(bytes[index]);
  }
  return output.str();
}

std::filesystem::path executable_path() {
  std::vector<wchar_t> buffer(32'768);
  const DWORD size = GetModuleFileNameW(nullptr, buffer.data(), static_cast<DWORD>(buffer.size()));
  if (size == 0 || size >= buffer.size()) fail("The observer executable path is unavailable.");
  return std::filesystem::path(std::wstring(buffer.data(), size));
}

void delete_fixed_file(const std::filesystem::path& file) {
  if (DeleteFileW(file.c_str()) == 0) {
    const DWORD error = GetLastError();
    if (error != ERROR_FILE_NOT_FOUND && error != ERROR_PATH_NOT_FOUND) {
      fail("A stale fixed external-observer evidence file could not be removed: path=" +
           utf8(file.wstring()) + " win32=" + std::to_string(error));
    }
  }
}

void write_new_file(const std::filesystem::path& file, std::string_view bytes) {
  UniqueHandle handle(CreateFileW(file.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                                  FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH, nullptr));
  if (!handle.valid()) fail("A fixed external-observer evidence file could not be created.");
  std::size_t offset = 0;
  while (offset < bytes.size()) {
    const DWORD chunk = static_cast<DWORD>(std::min<std::size_t>(bytes.size() - offset, 1U << 20U));
    DWORD written = 0;
    if (WriteFile(handle.value, bytes.data() + offset, chunk, &written, nullptr) == 0 || written != chunk) {
      fail("A fixed external-observer evidence file could not be written completely.");
    }
    offset += written;
  }
  if (FlushFileBuffers(handle.value) == 0) fail("A fixed external-observer evidence file could not be flushed.");
}

std::wstring sha256_file(const std::filesystem::path& file) {
  UniqueHandle input(CreateFileW(file.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
                                 FILE_FLAG_SEQUENTIAL_SCAN | FILE_FLAG_OPEN_REPARSE_POINT, nullptr));
  if (!input.valid()) fail("A fixed observer input could not be opened for hashing.");
  BY_HANDLE_FILE_INFORMATION information{};
  if (GetFileInformationByHandle(input.value, &information) == 0 ||
      (information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0 ||
      (information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0) {
    fail("A fixed observer input is not a non-reparse regular file.");
  }
  BCRYPT_ALG_HANDLE algorithm = nullptr;
  BCRYPT_HASH_HANDLE hash = nullptr;
  if (BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0) != 0) {
    fail("Windows SHA-256 initialization failed.");
  }
  DWORD object_size = 0;
  DWORD result_size = 0;
  if (BCryptGetProperty(algorithm, BCRYPT_OBJECT_LENGTH,
                        reinterpret_cast<PUCHAR>(&object_size), sizeof(object_size), &result_size, 0) != 0) {
    BCryptCloseAlgorithmProvider(algorithm, 0);
    fail("Windows SHA-256 object sizing failed.");
  }
  std::vector<BYTE> object(object_size);
  std::array<BYTE, 32> digest{};
  if (BCryptCreateHash(algorithm, &hash, object.data(), object_size, nullptr, 0, 0) != 0) {
    BCryptCloseAlgorithmProvider(algorithm, 0);
    fail("Windows SHA-256 hash creation failed.");
  }
  // The Windows GUI entrypoint has a small default stack. Keep the streaming
  // hash buffer on the heap so fixed-input admission cannot overflow it.
  std::vector<BYTE> buffer(1U << 20U);
  for (;;) {
    DWORD read = 0;
    if (ReadFile(input.value, buffer.data(), static_cast<DWORD>(buffer.size()), &read, nullptr) == 0) {
      BCryptDestroyHash(hash);
      BCryptCloseAlgorithmProvider(algorithm, 0);
      fail("A fixed observer input could not be read for hashing.");
    }
    if (read == 0) break;
    if (BCryptHashData(hash, buffer.data(), read, 0) != 0) {
      BCryptDestroyHash(hash);
      BCryptCloseAlgorithmProvider(algorithm, 0);
      fail("Windows SHA-256 hashing failed.");
    }
  }
  if (BCryptFinishHash(hash, digest.data(), static_cast<ULONG>(digest.size()), 0) != 0) {
    BCryptDestroyHash(hash);
    BCryptCloseAlgorithmProvider(algorithm, 0);
    fail("Windows SHA-256 finalization failed.");
  }
  BCryptDestroyHash(hash);
  BCryptCloseAlgorithmProvider(algorithm, 0);
  const auto hex = hex_bytes(digest.data(), digest.size());
  return std::wstring(hex.begin(), hex.end());
}

std::uint64_t file_bytes(const std::filesystem::path& file) {
  UniqueHandle handle(CreateFileW(file.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
                                  FILE_FLAG_OPEN_REPARSE_POINT, nullptr));
  if (!handle.valid()) fail("A fixed evidence file could not be opened for sizing.");
  LARGE_INTEGER size{};
  if (GetFileSizeEx(handle.value, &size) == 0 || size.QuadPart <= 0) {
    fail("A fixed evidence file has an invalid size.");
  }
  return static_cast<std::uint64_t>(size.QuadPart);
}

struct Moment {
  ULONGLONG file_time;
  std::string utc;
};

Moment now() {
  FILETIME file_time{};
  GetSystemTimePreciseAsFileTime(&file_time);
  ULARGE_INTEGER raw{};
  raw.LowPart = file_time.dwLowDateTime;
  raw.HighPart = file_time.dwHighDateTime;
  SYSTEMTIME system{};
  if (FileTimeToSystemTime(&file_time, &system) == 0) fail("A capture timestamp could not be formatted.");
  std::ostringstream output;
  output << std::setfill('0') << std::setw(4) << system.wYear << '-' << std::setw(2) << system.wMonth
         << '-' << std::setw(2) << system.wDay << 'T' << std::setw(2) << system.wHour << ':'
         << std::setw(2) << system.wMinute << ':' << std::setw(2) << system.wSecond << '.'
         << std::setw(3) << system.wMilliseconds << 'Z';
  return {raw.QuadPart, output.str()};
}

std::wstring registered_provider_name(const GUID& target) {
  ULONG bytes = 0;
  ULONG status = TdhEnumerateProviders(nullptr, &bytes);
  if (status != ERROR_INSUFFICIENT_BUFFER || bytes < sizeof(PROVIDER_ENUMERATION_INFO)) {
    fail("Required ETW provider enumeration is unavailable.");
  }
  std::vector<BYTE> buffer(bytes);
  auto* providers = reinterpret_cast<PROVIDER_ENUMERATION_INFO*>(buffer.data());
  status = TdhEnumerateProviders(providers, &bytes);
  if (status != ERROR_SUCCESS) fail("Required ETW provider enumeration failed.");
  for (ULONG index = 0; index < providers->NumberOfProviders; ++index) {
    const auto& row = providers->TraceProviderInfoArray[index];
    if (IsEqualGUID(row.ProviderGuid, target)) {
      if (row.ProviderNameOffset == 0 || row.ProviderNameOffset >= bytes) {
        fail("A required ETW provider lacks registered metadata.");
      }
      return reinterpret_cast<const wchar_t*>(buffer.data() + row.ProviderNameOffset);
    }
  }
  fail("A required ETW provider is not registered on this Windows host.");
}

std::wstring metadata_string(const TRACE_EVENT_INFO* info, ULONG offset) {
  if (offset == 0) return {};
  return reinterpret_cast<const wchar_t*>(reinterpret_cast<const BYTE*>(info) + offset);
}

struct EventDescriptorMetadata {
  USHORT id;
  UCHAR version;
  UCHAR level;
  UCHAR opcode;
  USHORT task;
  ULONGLONG keyword;
  std::wstring event_name;
  std::wstring opcode_name;
  std::wstring task_name;
};

struct ProviderMetadata {
  const ProviderDefinition* definition;
  std::wstring provider_name;
  std::vector<EventDescriptorMetadata> events;
  std::uint64_t observed_events{0};
  ULONG enable_status{ERROR_INVALID_STATE};
};

ProviderMetadata provider_metadata(const ProviderDefinition& definition) {
  ProviderMetadata output{&definition, registered_provider_name(*definition.id)};
  ULONG bytes = 0;
  ULONG status = TdhEnumerateManifestProviderEvents(
      const_cast<GUID*>(definition.id), nullptr, &bytes);
  if (status != ERROR_INSUFFICIENT_BUFFER || bytes < sizeof(PROVIDER_EVENT_INFO)) {
    fail("A required ETW provider has no enumerable manifest event metadata.");
  }
  std::vector<BYTE> event_buffer(bytes);
  auto* enumeration = reinterpret_cast<PROVIDER_EVENT_INFO*>(event_buffer.data());
  status = TdhEnumerateManifestProviderEvents(
      const_cast<GUID*>(definition.id), enumeration, &bytes);
  if (status != ERROR_SUCCESS || enumeration->NumberOfEvents == 0) {
    fail("A required ETW provider manifest event inventory is unavailable.");
  }
  output.events.reserve(enumeration->NumberOfEvents);
  for (ULONG index = 0; index < enumeration->NumberOfEvents; ++index) {
    const EVENT_DESCRIPTOR descriptor = enumeration->EventDescriptorsArray[index];
    ULONG info_bytes = 0;
    status = TdhGetManifestEventInformation(
        const_cast<GUID*>(definition.id), const_cast<EVENT_DESCRIPTOR*>(&descriptor), nullptr, &info_bytes);
    if (status != ERROR_INSUFFICIENT_BUFFER || info_bytes < sizeof(TRACE_EVENT_INFO)) {
      fail("A required ETW event descriptor lacks manifest metadata.");
    }
    std::vector<BYTE> info_buffer(info_bytes);
    auto* info = reinterpret_cast<TRACE_EVENT_INFO*>(info_buffer.data());
    status = TdhGetManifestEventInformation(
        const_cast<GUID*>(definition.id), const_cast<EVENT_DESCRIPTOR*>(&descriptor), info, &info_bytes);
    if (status != ERROR_SUCCESS) fail("A required ETW event manifest could not be read.");
    output.events.push_back({
        descriptor.Id,
        descriptor.Version,
        descriptor.Level,
        descriptor.Opcode,
        descriptor.Task,
        descriptor.Keyword,
        metadata_string(info, info->EventMessageOffset),
        metadata_string(info, info->OpcodeNameOffset),
        metadata_string(info, info->TaskNameOffset),
    });
  }
  std::sort(output.events.begin(), output.events.end(), [](const auto& left, const auto& right) {
    return std::tie(left.id, left.version) < std::tie(right.id, right.version);
  });
  for (std::size_t index = 1; index < output.events.size(); ++index) {
    if (output.events[index - 1].id == output.events[index].id &&
        output.events[index - 1].version == output.events[index].version) {
      fail("A required ETW provider repeats an event ID/version descriptor.");
    }
  }
  return output;
}

template <typename Integer>
std::string integer_value(const std::vector<BYTE>& bytes) {
  if (bytes.size() < sizeof(Integer)) return {};
  Integer value{};
  std::memcpy(&value, bytes.data(), sizeof(value));
  if constexpr (std::is_signed_v<Integer>) {
    return std::to_string(static_cast<long long>(value));
  } else {
    return std::to_string(static_cast<unsigned long long>(value));
  }
}

std::string ipv6_value(const BYTE* bytes) {
  std::ostringstream output;
  output << std::hex << std::nouppercase;
  for (std::size_t index = 0; index < 8; ++index) {
    if (index != 0) output << ':';
    const unsigned word = (static_cast<unsigned>(bytes[index * 2]) << 8U) |
                          static_cast<unsigned>(bytes[index * 2 + 1]);
    output << word;
  }
  return output.str();
}

std::string property_value(USHORT type, USHORT out_type, const std::vector<BYTE>& bytes) {
  if (bytes.empty()) return {};
  if (out_type == TDH_OUTTYPE_IPV4) {
    if (bytes.size() < 4) return {};
    return std::to_string(bytes[0]) + "." + std::to_string(bytes[1]) + "." +
           std::to_string(bytes[2]) + "." + std::to_string(bytes[3]);
  }
  if (out_type == TDH_OUTTYPE_IPV6) return bytes.size() >= 16 ? ipv6_value(bytes.data()) : std::string{};
  if (out_type == TDH_OUTTYPE_SOCKETADDRESS) {
    if (bytes.size() < 4) return {};
    std::uint16_t family = 0;
    std::memcpy(&family, bytes.data(), sizeof(family));
    const unsigned port = (static_cast<unsigned>(bytes[2]) << 8U) | bytes[3];
    if (family == 2 && bytes.size() >= 8) {
      return std::to_string(bytes[4]) + "." + std::to_string(bytes[5]) + "." +
             std::to_string(bytes[6]) + "." + std::to_string(bytes[7]) + ":" +
             std::to_string(port);
    }
    if (family == 23 && bytes.size() >= 24) {
      return "[" + ipv6_value(bytes.data() + 8) + "]:" + std::to_string(port);
    }
    return {};
  }
  switch (type) {
    case TDH_INTYPE_UNICODESTRING: {
      const auto* value = reinterpret_cast<const wchar_t*>(bytes.data());
      const std::size_t count = bytes.size() / sizeof(wchar_t);
      std::size_t length = 0;
      while (length < count && value[length] != L'\0') ++length;
      return utf8(std::wstring_view(value, length));
    }
    case TDH_INTYPE_ANSISTRING: {
      const auto* value = reinterpret_cast<const char*>(bytes.data());
      std::size_t length = 0;
      while (length < bytes.size() && value[length] != '\0') ++length;
      return std::string(value, length);
    }
    case TDH_INTYPE_INT8: return integer_value<std::int8_t>(bytes);
    case TDH_INTYPE_UINT8: return integer_value<std::uint8_t>(bytes);
    case TDH_INTYPE_INT16: return integer_value<std::int16_t>(bytes);
    case TDH_INTYPE_UINT16: return integer_value<std::uint16_t>(bytes);
    case TDH_INTYPE_INT32: return integer_value<std::int32_t>(bytes);
    case TDH_INTYPE_UINT32:
    case TDH_INTYPE_HEXINT32:
    case TDH_INTYPE_BOOLEAN: return integer_value<std::uint32_t>(bytes);
    case TDH_INTYPE_INT64: return integer_value<std::int64_t>(bytes);
    case TDH_INTYPE_UINT64:
    case TDH_INTYPE_HEXINT64: return integer_value<std::uint64_t>(bytes);
    default: return hex_bytes(bytes.data(), bytes.size());
  }
}

struct Property {
  std::wstring name;
  std::string value;
};

std::vector<Property> event_properties(PEVENT_RECORD record, const TRACE_EVENT_INFO* info) {
  std::vector<Property> output;
  std::set<std::wstring> names;
  const ULONG property_count = info->TopLevelPropertyCount;
  for (ULONG index = 0; index < property_count; ++index) {
    const EVENT_PROPERTY_INFO& property = info->EventPropertyInfoArray[index];
    if ((property.Flags & PropertyStruct) != 0 || property.NameOffset == 0) continue;
    const auto* name = reinterpret_cast<const wchar_t*>(
        reinterpret_cast<const BYTE*>(info) + property.NameOffset);
    const std::wstring canonical = lower(name);
    if (!names.insert(canonical).second) continue;
    PROPERTY_DATA_DESCRIPTOR descriptor{};
    descriptor.PropertyName = reinterpret_cast<ULONGLONG>(name);
    descriptor.ArrayIndex = ULONG_MAX;
    ULONG size = 0;
    ULONG status = TdhGetPropertySize(record, 0, nullptr, 1, &descriptor, &size);
    if (status != ERROR_SUCCESS || size == 0 || size > 8192) continue;
    std::vector<BYTE> bytes(size);
    status = TdhGetProperty(record, 0, nullptr, 1, &descriptor, size, bytes.data());
    if (status != ERROR_SUCCESS) continue;
    std::string value = property_value(
        property.nonStructType.InType, property.nonStructType.OutType, bytes);
    if (!value.empty() && value.size() <= 8192) output.push_back({name, std::move(value)});
  }
  return output;
}

std::optional<std::string> find_property(const std::vector<Property>& properties,
                                         std::initializer_list<std::wstring_view> names) {
  for (const auto name : names) {
    const std::wstring wanted = lower(std::wstring(name));
    for (const auto& property : properties) {
      if (lower(property.name) == wanted) return property.value;
    }
  }
  return std::nullopt;
}

std::optional<DWORD> numeric_property(const std::vector<Property>& properties,
                                      std::initializer_list<std::wstring_view> names) {
  const auto value = find_property(properties, names);
  if (!value || value->empty() || value->size() > 10 ||
      !std::all_of(value->begin(), value->end(), [](char character) { return character >= '0' && character <= '9'; })) {
    return std::nullopt;
  }
  const unsigned long parsed = std::stoul(*value);
  if (parsed > MAXDWORD) return std::nullopt;
  return static_cast<DWORD>(parsed);
}

void set_property(std::vector<Property>& properties, std::wstring name, std::string value) {
  const std::wstring wanted = lower(name);
  for (auto& property : properties) {
    if (lower(property.name) == wanted) {
      property.name = std::move(name);
      property.value = std::move(value);
      return;
    }
  }
  properties.push_back({std::move(name), std::move(value)});
}

std::optional<std::filesystem::path> process_image_path(DWORD process_id) {
  UniqueHandle process(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, process_id));
  if (!process.valid()) return std::nullopt;
  std::vector<wchar_t> buffer(32'768);
  DWORD size = static_cast<DWORD>(buffer.size());
  if (QueryFullProcessImageNameW(process.value, 0, buffer.data(), &size) == 0 || size == 0) return std::nullopt;
  return std::filesystem::path(std::wstring(buffer.data(), size));
}

bool includes(std::wstring_view haystack, std::wstring_view needle) {
  return haystack.find(needle) != std::wstring_view::npos;
}

enum class EventKind { dns, network_passive, other, packet, process_start, process_stop, socket };

const char* event_kind_text(EventKind kind) {
  switch (kind) {
    case EventKind::dns: return "DNS_RESOLVER";
    case EventKind::network_passive: return "NETWORK_PASSIVE";
    case EventKind::packet: return "PACKET";
    case EventKind::process_start: return "PROCESS_START";
    case EventKind::process_stop: return "PROCESS_STOP";
    case EventKind::socket: return "SOCKET";
    default: return "OTHER";
  }
}

EventKind classify(const GUID& provider, UCHAR opcode, std::wstring_view names) {
  if (IsEqualGUID(provider, kKernelProcess)) {
    if (opcode == EVENT_TRACE_TYPE_STOP) return EventKind::process_stop;
    if (opcode == EVENT_TRACE_TYPE_START) return EventKind::process_start;
    return EventKind::other;
  }
  if (IsEqualGUID(provider, kNameResolution) || IsEqualGUID(provider, kDnsClient)) {
    if (includes(names, L"config") || includes(names, L"cache") || includes(names, L"response") ||
        includes(names, L"complete") || includes(names, L"stop")) {
      return EventKind::network_passive;
    }
    return EventKind::dns;
  }
  if (IsEqualGUID(provider, kWinsockAfd)) {
    if (includes(names, L"connect") || includes(names, L"send") || includes(names, L"bind") ||
        includes(names, L"listen")) return EventKind::socket;
    if (includes(names, L"recv") || includes(names, L"receive") || includes(names, L"accept") ||
        includes(names, L"disconnect") || includes(names, L"close")) return EventKind::network_passive;
    return EventKind::other;
  }
  if (IsEqualGUID(provider, kKernelNetwork)) {
    if (includes(names, L"send") || includes(names, L"transmit")) return EventKind::packet;
    if (includes(names, L"recv") || includes(names, L"receive")) return EventKind::network_passive;
  }
  return EventKind::other;
}

struct ConsumerContext {
  std::filesystem::path event_path;
  std::ofstream stream;
  std::mutex mutex;
  std::condition_variable ready_condition;
  bool ready{false};
  ULONG process_trace_status{ERROR_INVALID_STATE};
  std::uint64_t sequence{0};
  std::map<std::wstring, std::uint64_t> provider_counts;
  std::set<DWORD> candidate_descendants;
  std::atomic<DWORD> launcher_process_id{0};
};

std::wstring trace_info_name(const TRACE_EVENT_INFO* info, ULONG offset) {
  return offset == 0 ? std::wstring{} :
      std::wstring(reinterpret_cast<const wchar_t*>(reinterpret_cast<const BYTE*>(info) + offset));
}

void WINAPI event_callback(PEVENT_RECORD record) {
  auto* context = static_cast<ConsumerContext*>(record->UserContext);
  if (context == nullptr) return;
  try {
    const bool required = std::any_of(kProviders.begin(), kProviders.end(), [&](const auto& provider) {
      return IsEqualGUID(record->EventHeader.ProviderId, *provider.id);
    });
    if (!required) return;
    ULONG info_bytes = 0;
    ULONG status = TdhGetEventInformation(record, 0, nullptr, nullptr, &info_bytes);
    if (status != ERROR_INSUFFICIENT_BUFFER || info_bytes < sizeof(TRACE_EVENT_INFO)) {
      context->process_trace_status = ERROR_INVALID_DATA;
      return;
    }
    std::vector<BYTE> info_buffer(info_bytes);
    auto* info = reinterpret_cast<TRACE_EVENT_INFO*>(info_buffer.data());
    status = TdhGetEventInformation(record, 0, nullptr, info, &info_bytes);
    if (status != ERROR_SUCCESS) {
      context->process_trace_status = status;
      return;
    }
    const std::wstring event_name = trace_info_name(info, info->EventMessageOffset);
    const std::wstring opcode_name = trace_info_name(info, info->OpcodeNameOffset);
    const std::wstring task_name = trace_info_name(info, info->TaskNameOffset);
    const std::wstring combined = lower(event_name + L" " + opcode_name + L" " + task_name);
    const EventKind kind = classify(
        record->EventHeader.ProviderId,
        record->EventHeader.EventDescriptor.Opcode,
        combined);
    auto properties = event_properties(record, info);
    if (kind == EventKind::process_start || kind == EventKind::process_stop) {
      const auto pid = numeric_property(properties, {L"ProcessId", L"ProcessID", L"PID"});
      if (pid) set_property(properties, L"ObserverProcessId", std::to_string(*pid));
      if (kind == EventKind::process_start && pid) {
        const auto parent = numeric_property(properties,
            {L"ParentProcessId", L"ParentProcessID", L"ParentId", L"PPID"});
        if (parent) set_property(properties, L"ObserverParentProcessId", std::to_string(*parent));
        const DWORD launcher = context->launcher_process_id.load();
        if (parent && (launcher != 0 && *parent == launcher || context->candidate_descendants.contains(*parent))) {
          context->candidate_descendants.insert(*pid);
          if (const auto image = process_image_path(*pid)) {
            set_property(properties, L"ObserverImageSha256", utf8(sha256_file(*image)));
          }
        }
      }
    } else {
      const auto payload_pid = numeric_property(properties, {L"ProcessId", L"ProcessID", L"PID"});
      const DWORD attributed_pid = payload_pid.value_or(record->EventHeader.ProcessId);
      if (attributed_pid != 0) set_property(properties, L"ObserverProcessId", std::to_string(attributed_pid));
      if (kind == EventKind::socket || kind == EventKind::packet) {
        const bool listener = includes(combined, L"listen") || includes(combined, L"bind");
        set_property(properties, L"ObserverDirection", listener ? "listen" : "outbound");
        const auto target = find_property(properties,
            {L"RemoteAddress", L"DestinationAddress", L"DestAddress", L"daddr", L"Address"});
        if (target) set_property(properties, L"ObserverTargetAddress", *target);
      }
    }
    std::sort(properties.begin(), properties.end(), [](const auto& left, const auto& right) {
      return lower(left.name) < lower(right.name);
    });
    const std::wstring provider_id = guid_text(record->EventHeader.ProviderId);
    context->provider_counts[provider_id] += 1;
    const std::uint64_t sequence = ++context->sequence;
    std::ostringstream output;
    output << "{\"eventId\":" << record->EventHeader.EventDescriptor.Id
           << ",\"eventName\":\"" << json_escape(utf8(event_name)) << "\""
           << ",\"headerProcessId\":" << record->EventHeader.ProcessId
           << ",\"headerThreadId\":" << record->EventHeader.ThreadId
           << ",\"keyword\":\"" << hex_u64(record->EventHeader.EventDescriptor.Keyword) << "\""
           << ",\"kind\":\"" << event_kind_text(kind) << "\""
           << ",\"level\":" << static_cast<unsigned>(record->EventHeader.EventDescriptor.Level)
           << ",\"opcode\":" << static_cast<unsigned>(record->EventHeader.EventDescriptor.Opcode)
           << ",\"opcodeName\":\"" << json_escape(utf8(opcode_name)) << "\""
           << ",\"properties\":[";
    for (std::size_t index = 0; index < properties.size(); ++index) {
      if (index != 0) output << ',';
      output << "{\"name\":\"" << json_escape(utf8(properties[index].name))
             << "\",\"value\":\"" << json_escape(properties[index].value) << "\"}";
    }
    output << "]"
           << ",\"providerId\":\"" << utf8(provider_id) << "\""
           << ",\"sequence\":" << sequence
           << ",\"task\":" << record->EventHeader.EventDescriptor.Task
           << ",\"taskName\":\"" << json_escape(utf8(task_name)) << "\""
           << ",\"timestampFileTime\":\"" << record->EventHeader.TimeStamp.QuadPart << "\""
           << ",\"version\":" << static_cast<unsigned>(record->EventHeader.EventDescriptor.Version)
           << "}\n";
    context->stream << output.str();
    if (!context->stream) context->process_trace_status = ERROR_WRITE_FAULT;
  } catch (...) {
    context->process_trace_status = ERROR_INVALID_DATA;
  }
}

class TraceSession {
 public:
  TraceSession(const std::filesystem::path& etl, ConsumerContext& context)
      : context_(context), properties_(properties_size()) {
    auto* properties = trace_properties();
    std::memset(properties, 0, properties_.size());
    properties->Wnode.BufferSize = static_cast<ULONG>(properties_.size());
    properties->Wnode.Flags = WNODE_FLAG_TRACED_GUID;
    properties->Wnode.ClientContext = 2;
    properties->Wnode.Guid = kSessionGuid;
    properties->BufferSize = 64;
    properties->MinimumBuffers = 128;
    properties->MaximumBuffers = 512;
    properties->MaximumFileSize = 2048;
    properties->FlushTimer = 1;
    properties->LogFileMode = EVENT_TRACE_FILE_MODE_SEQUENTIAL |
                              EVENT_TRACE_REAL_TIME_MODE |
                              EVENT_TRACE_SYSTEM_LOGGER_MODE |
                              EVENT_TRACE_NO_PER_PROCESSOR_BUFFERING;
    properties->LoggerNameOffset = sizeof(EVENT_TRACE_PROPERTIES);
    properties->LogFileNameOffset = sizeof(EVENT_TRACE_PROPERTIES) +
        static_cast<ULONG>((std::wcslen(kSessionName) + 1) * sizeof(wchar_t));
    std::memcpy(properties_.data() + properties->LoggerNameOffset, kSessionName,
                (std::wcslen(kSessionName) + 1) * sizeof(wchar_t));
    const std::wstring etl_text = etl.wstring();
    if (etl_text.size() > 32'000) fail("The fixed ETL path is too long.");
    std::memcpy(properties_.data() + properties->LogFileNameOffset, etl_text.c_str(),
                (etl_text.size() + 1) * sizeof(wchar_t));
    const ULONG status = StartTraceW(&handle_, kSessionName, properties);
    if (status == ERROR_ALREADY_EXISTS) fail("The fixed ETW observer session already exists; it was not modified.");
    if (status != ERROR_SUCCESS) {
      fail("The fixed ETW observer session could not start; win32=" + std::to_string(status) + ".");
    }
  }

  TraceSession(const TraceSession&) = delete;
  TraceSession& operator=(const TraceSession&) = delete;
  ~TraceSession() noexcept {
    stop_trace_and_consumer();
  }

  void start_consumer() {
    context_.stream.open(context_.event_path, std::ios::binary | std::ios::trunc);
    if (!context_.stream) fail("The normalized ETW event stream could not be created.");
    consumer_ = std::thread([this] {
      EVENT_TRACE_LOGFILEW log{};
      log.LoggerName = const_cast<LPWSTR>(kSessionName);
      log.ProcessTraceMode = PROCESS_TRACE_MODE_EVENT_RECORD |
                             PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_RAW_TIMESTAMP;
      log.EventRecordCallback = event_callback;
      log.Context = &context_;
      TRACEHANDLE consumer = OpenTraceW(&log);
      {
        std::lock_guard lock(consumer_mutex_);
        consumer_handle_ = consumer;
      }
      {
        std::lock_guard lock(context_.mutex);
        context_.ready = true;
      }
      context_.ready_condition.notify_all();
      if (consumer == INVALID_PROCESSTRACE_HANDLE) {
        context_.process_trace_status = GetLastError();
        return;
      }
      const ULONG status = ProcessTrace(&consumer, 1, nullptr, nullptr);
      if (status != ERROR_SUCCESS && status != ERROR_CANCELLED && context_.process_trace_status == ERROR_INVALID_STATE) {
        context_.process_trace_status = status;
      }
      bool close_consumer = false;
      {
        std::lock_guard lock(consumer_mutex_);
        if (consumer_handle_ == consumer) {
          consumer_handle_ = INVALID_PROCESSTRACE_HANDLE;
          close_consumer = true;
        }
      }
      if (close_consumer) CloseTrace(consumer);
    });
    std::unique_lock lock(context_.mutex);
    if (!context_.ready_condition.wait_for(lock, std::chrono::seconds(10), [this] { return context_.ready; }) ||
        !consumer_attached()) {
      fail("The real-time ETW consumer did not attach before provider enablement.");
    }
  }

  void enable(std::vector<ProviderMetadata>& metadata) {
    ENABLE_TRACE_PARAMETERS parameters{};
    parameters.Version = ENABLE_TRACE_PARAMETERS_VERSION_2;
    for (auto& provider : metadata) {
      provider.enable_status = EnableTraceEx2(
          handle_, provider.definition->id, EVENT_CONTROL_CODE_ENABLE_PROVIDER,
          TRACE_LEVEL_VERBOSE, provider.definition->match_any_keyword, 0, 0, &parameters);
      if (provider.enable_status != ERROR_SUCCESS) fail("A required ETW provider could not be enabled.");
    }
  }

  EVENT_TRACE_PROPERTIES stop() {
    const ULONG status = stop_trace_and_consumer();
    if (status != ERROR_SUCCESS) fail("The fixed ETW observer session could not stop cleanly.");
    if (context_.process_trace_status != ERROR_INVALID_STATE &&
        context_.process_trace_status != ERROR_SUCCESS) {
      fail("The real-time ETW consumer reported an incomplete event stream.");
    }
    return *trace_properties();
  }

 private:
  [[nodiscard]] bool consumer_attached() {
    std::lock_guard lock(consumer_mutex_);
    return consumer_handle_ != INVALID_PROCESSTRACE_HANDLE;
  }

  ULONG stop_trace_and_consumer() noexcept {
    ULONG status = ERROR_SUCCESS;
    if (active_) {
      status = ControlTraceW(handle_, kSessionName, trace_properties(), EVENT_TRACE_CONTROL_STOP);
      active_ = false;
    }
    if (status != ERROR_SUCCESS) {
      TRACEHANDLE consumer = INVALID_PROCESSTRACE_HANDLE;
      {
        std::lock_guard lock(consumer_mutex_);
        consumer = std::exchange(consumer_handle_, INVALID_PROCESSTRACE_HANDLE);
      }
      if (consumer != INVALID_PROCESSTRACE_HANDLE) CloseTrace(consumer);
    }
    if (consumer_.joinable()) consumer_.join();
    if (context_.stream.is_open()) {
      context_.stream.flush();
      context_.stream.close();
    }
    return status;
  }

  static std::size_t properties_size() {
    return sizeof(EVENT_TRACE_PROPERTIES) +
           (std::wcslen(kSessionName) + 1 + 32'768) * sizeof(wchar_t);
  }
  EVENT_TRACE_PROPERTIES* trace_properties() {
    return reinterpret_cast<EVENT_TRACE_PROPERTIES*>(properties_.data());
  }
  ConsumerContext& context_;
  std::vector<BYTE> properties_;
  TRACEHANDLE handle_{0};
  TRACEHANDLE consumer_handle_{INVALID_PROCESSTRACE_HANDLE};
  std::mutex consumer_mutex_;
  std::thread consumer_;
  bool active_{true};
};

struct LaunchedProcess {
  UniqueHandle process;
  UniqueHandle thread;
  DWORD process_id;
};

std::vector<BYTE> token_information(
    HANDLE token,
    TOKEN_INFORMATION_CLASS information_class) {
  DWORD bytes = 0;
  SetLastError(ERROR_SUCCESS);
  const BOOL sized = GetTokenInformation(token, information_class, nullptr, 0, &bytes);
  const DWORD size_status = GetLastError();
  if (sized != 0 || size_status != ERROR_INSUFFICIENT_BUFFER ||
      bytes == 0 || bytes > 1024U * 1024U) {
    fail("A fixed launcher token identity size failed closed; class=" +
         std::to_string(static_cast<int>(information_class)) +
         ", win32=" + std::to_string(size_status) + ".");
  }
  std::vector<BYTE> result(bytes);
  if (GetTokenInformation(
      token, information_class, result.data(), bytes, &bytes) == 0 ||
      bytes == 0 || bytes > result.size()) {
    fail("A fixed launcher token identity could not be read; class=" +
         std::to_string(static_cast<int>(information_class)) +
         ", win32=" + std::to_string(GetLastError()) + ".");
  }
  result.resize(bytes);
  return result;
}

template <typename Value>
Value token_scalar(HANDLE token, TOKEN_INFORMATION_CLASS information_class) {
  Value value{};
  DWORD bytes = 0;
  if (GetTokenInformation(
          token,
          information_class,
          &value,
          sizeof(value),
          &bytes) == 0 ||
      bytes != sizeof(value)) {
    fail("A fixed launcher token scalar could not be read; class=" +
         std::to_string(static_cast<int>(information_class)) +
         ", win32=" + std::to_string(GetLastError()) + ".");
  }
  return value;
}

std::vector<BYTE> token_user_sid(HANDLE token) {
  const auto bytes = token_information(token, TokenUser);
  const auto* user = reinterpret_cast<const TOKEN_USER*>(bytes.data());
  if (user->User.Sid == nullptr || IsValidSid(user->User.Sid) == 0) {
    fail("A fixed launcher user SID is malformed.");
  }
  const DWORD length = GetLengthSid(user->User.Sid);
  std::vector<BYTE> result(length);
  if (length == 0 || CopySid(length, result.data(), user->User.Sid) == 0) {
    fail("A fixed launcher user SID could not be retained.");
  }
  return result;
}

std::vector<BYTE> token_logon_sid(HANDLE token) {
  const auto bytes = token_information(token, TokenGroups);
  const auto* groups = reinterpret_cast<const TOKEN_GROUPS*>(bytes.data());
  std::vector<BYTE> result;
  for (DWORD index = 0; index < groups->GroupCount; ++index) {
    const auto& group = groups->Groups[index];
    if ((group.Attributes & SE_GROUP_LOGON_ID) != SE_GROUP_LOGON_ID) continue;
    if (!result.empty() || group.Sid == nullptr || IsValidSid(group.Sid) == 0) {
      fail("A fixed launcher logon SID is ambiguous.");
    }
    const DWORD length = GetLengthSid(group.Sid);
    result.resize(length);
    if (length == 0 || CopySid(length, result.data(), group.Sid) == 0) {
      fail("A fixed launcher logon SID could not be retained.");
    }
  }
  if (result.empty()) fail("A fixed launcher logon SID is unavailable.");
  return result;
}

bool equal_sid_bytes(const std::vector<BYTE>& left, const std::vector<BYTE>& right) {
  return !left.empty() && !right.empty() &&
      IsValidSid(const_cast<BYTE*>(left.data())) != 0 &&
      IsValidSid(const_cast<BYTE*>(right.data())) != 0 &&
      EqualSid(const_cast<BYTE*>(left.data()), const_cast<BYTE*>(right.data())) != 0;
}

void require_same_token_identity(HANDLE left, HANDLE right) {
  if (!equal_sid_bytes(token_user_sid(left), token_user_sid(right)) ||
      !equal_sid_bytes(token_logon_sid(left), token_logon_sid(right)) ||
      token_scalar<DWORD>(left, TokenSessionId) !=
          token_scalar<DWORD>(right, TokenSessionId)) {
    fail("The fixed launcher token does not match the interactive user and session.");
  }
}

void require_standard_interactive_token(HANDLE token, bool require_primary = true) {
  const auto type = token_scalar<TOKEN_TYPE>(token, TokenType);
  const auto elevation_type =
      token_scalar<TOKEN_ELEVATION_TYPE>(token, TokenElevationType);
  const auto elevation = token_scalar<TOKEN_ELEVATION>(token, TokenElevation);
  const auto app_container = token_scalar<DWORD>(token, TokenIsAppContainer);
  const auto ui_access = token_scalar<DWORD>(token, TokenUIAccess);
  const auto integrity_bytes = token_information(token, TokenIntegrityLevel);
  const auto* integrity =
      reinterpret_cast<const TOKEN_MANDATORY_LABEL*>(integrity_bytes.data());
  if (integrity->Label.Sid == nullptr || IsValidSid(integrity->Label.Sid) == 0) {
    fail("The fixed launcher integrity SID is malformed.");
  }
  const auto* count = GetSidSubAuthorityCount(integrity->Label.Sid);
  if (count == nullptr || *count == 0) {
    fail("The fixed launcher integrity level is unavailable.");
  }
  const DWORD integrity_rid =
      *GetSidSubAuthority(integrity->Label.Sid, static_cast<DWORD>(*count - 1));
  const bool type_allowed = require_primary ? type == TokenPrimary :
      (type == TokenPrimary || type == TokenImpersonation);
  if (!type_allowed || elevation_type != TokenElevationTypeLimited ||
      elevation.TokenIsElevated != 0 || app_container != 0 || ui_access != 0 ||
      integrity_rid != SECURITY_MANDATORY_MEDIUM_RID) {
    fail("The fixed launcher token is not a standard medium-integrity desktop token; type=" +
         std::to_string(static_cast<int>(type)) + ", elevationType=" +
         std::to_string(static_cast<int>(elevation_type)) + ", elevated=" +
         std::to_string(elevation.TokenIsElevated) + ", appContainer=" +
         std::to_string(app_container) + ", uiAccess=" + std::to_string(ui_access) +
         ", integrityRid=" + std::to_string(integrity_rid) + ".");
  }
}

UniqueHandle open_token(HANDLE process, DWORD access) {
  HANDLE token = INVALID_HANDLE_VALUE;
  if (OpenProcessToken(process, access, &token) == 0) {
    fail("A fixed launcher process token could not be opened.");
  }
  return UniqueHandle(token);
}

UniqueHandle interactive_shell_token() {
  const HWND shell = GetShellWindow();
  DWORD process_id = 0;
  if (shell == nullptr || GetWindowThreadProcessId(shell, &process_id) == 0 ||
      process_id == 0) {
    fail("The interactive Windows shell identity is unavailable.");
  }
  UniqueHandle process(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, process_id));
  if (!process.valid()) fail("The interactive Windows shell could not be attested.");
  return open_token(process.value, TOKEN_QUERY);
}

UniqueHandle linked_standard_user_token() {
  auto elevated = open_token(GetCurrentProcess(), TOKEN_QUERY);
  if (token_scalar<TOKEN_ELEVATION_TYPE>(elevated.value, TokenElevationType) !=
      TokenElevationTypeFull) {
    fail("The ETW observer does not hold a full UAC token.");
  }
  TOKEN_LINKED_TOKEN linked{};
  DWORD linked_bytes = 0;
  if (GetTokenInformation(
          elevated.value,
          TokenLinkedToken,
          &linked,
          sizeof(linked),
          &linked_bytes) == 0 ||
      linked_bytes != sizeof(linked)) {
    fail("The linked standard-user token could not be read; win32=" +
         std::to_string(GetLastError()) + ".");
  }
  UniqueHandle linked_token(linked.LinkedToken);
  if (!linked_token.valid()) fail("The linked standard-user token is unavailable.");
  require_standard_interactive_token(linked_token.value, false);
  require_same_token_identity(elevated.value, linked_token.value);

  auto shell_token = interactive_shell_token();
  require_standard_interactive_token(shell_token.value);
  require_same_token_identity(linked_token.value, shell_token.value);

  HANDLE primary = INVALID_HANDLE_VALUE;
  if (DuplicateTokenEx(
          linked_token.value,
          TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY,
          nullptr,
          SecurityImpersonation,
          TokenPrimary,
          &primary) == 0) {
    fail("The linked standard-user primary token could not be created.");
  }
  UniqueHandle result(primary);
  require_standard_interactive_token(result.value);
  require_same_token_identity(result.value, shell_token.value);
  return result;
}

LaunchedProcess create_fixed_launcher(const std::filesystem::path& launcher) {
  auto standard_token = linked_standard_user_token();
  std::wstring command = L"\"" + launcher.wstring() + L"\"";
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  std::wstring desktop = L"winsta0\\default";
  startup.lpDesktop = desktop.data();
  PROCESS_INFORMATION process{};
  if (CreateProcessWithTokenW(
          standard_token.value,
          LOGON_WITH_PROFILE,
          launcher.c_str(),
          command.data(),
          CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW,
          nullptr,
          launcher.parent_path().c_str(),
          &startup,
          &process) == 0) {
    fail("The fixed exact-candidate launcher could not be created as the interactive standard user; win32=" +
         std::to_string(GetLastError()) + ".");
  }
  UniqueHandle process_handle(process.hProcess);
  UniqueHandle thread_handle(process.hThread);
  try {
    auto child_token = open_token(process_handle.value, TOKEN_QUERY);
    require_standard_interactive_token(child_token.value);
    require_same_token_identity(child_token.value, standard_token.value);
  } catch (...) {
    TerminateProcess(process_handle.value, 1);
    throw;
  }
  return {std::move(process_handle), std::move(thread_handle), process.dwProcessId};
}

DWORD run_fixed_launcher(const std::filesystem::path& launcher, ConsumerContext& context,
                         Moment& started, Moment& exited) {
  auto process = create_fixed_launcher(launcher);
  context.launcher_process_id.store(process.process_id);
  started = now();
  if (ResumeThread(process.thread.value) == static_cast<DWORD>(-1)) {
    TerminateProcess(process.process.value, 1);
    fail("The fixed exact-candidate launcher could not be resumed.");
  }
  process.thread.reset();
  const DWORD wait = WaitForSingleObject(process.process.value, 15U * 60U * 1000U);
  if (wait != WAIT_OBJECT_0) {
    TerminateProcess(process.process.value, 1);
    fail("The fixed exact-candidate launcher timed out or could not be observed.");
  }
  DWORD exit_code = 1;
  if (GetExitCodeProcess(process.process.value, &exit_code) == 0) {
    fail("The fixed exact-candidate launcher exit code is unavailable.");
  }
  exited = now();
  std::this_thread::sleep_for(std::chrono::seconds(3));
  return exit_code;
}

std::string provider_metadata_json(const std::vector<ProviderMetadata>& providers,
                                   const ConsumerContext& context) {
  std::ostringstream output;
  output << "{\n  \"evidenceKind\": \"WINDOWS_PHASE2_ETW_PROVIDER_METADATA\",\n"
         << "  \"providers\": [\n";
  for (std::size_t provider_index = 0; provider_index < providers.size(); ++provider_index) {
    const auto& provider = providers[provider_index];
    const std::wstring id = guid_text(*provider.definition->id);
    const auto count = context.provider_counts.contains(id) ? context.provider_counts.at(id) : 0;
    output << "    {\n      \"eventDescriptors\": [\n";
    for (std::size_t event_index = 0; event_index < provider.events.size(); ++event_index) {
      const auto& event = provider.events[event_index];
      output << "        {\"eventId\": " << event.id
             << ", \"eventName\": \"" << json_escape(utf8(event.event_name)) << "\""
             << ", \"keyword\": \"" << hex_u64(event.keyword) << "\""
             << ", \"level\": " << static_cast<unsigned>(event.level)
             << ", \"opcode\": " << static_cast<unsigned>(event.opcode)
             << ", \"opcodeName\": \"" << json_escape(utf8(event.opcode_name)) << "\""
             << ", \"task\": " << event.task
             << ", \"taskName\": \"" << json_escape(utf8(event.task_name)) << "\""
             << ", \"version\": " << static_cast<unsigned>(event.version) << "}"
             << (event_index + 1 == provider.events.size() ? "\n" : ",\n");
    }
    output << "      ],\n      \"manifestEventCount\": " << provider.events.size()
           << ",\n      \"observedEventCount\": " << count
           << ",\n      \"providerId\": \"" << utf8(id) << "\""
           << ",\n      \"providerName\": \"" << json_escape(utf8(provider.provider_name)) << "\""
           << ",\n      \"role\": \"" << json_escape(utf8(provider.definition->role)) << "\"\n    }"
           << (provider_index + 1 == providers.size() ? "\n" : ",\n");
  }
  output << "  ],\n  \"schemaVersion\": \"1.0\"\n}\n";
  return output.str();
}

struct EvidenceRow {
  std::string path;
  std::uint64_t bytes;
  std::string sha256;
};

EvidenceRow evidence_row(const std::filesystem::path& root, std::string path) {
  const auto file = root / std::filesystem::path(path);
  return {std::move(path), file_bytes(file), utf8(sha256_file(file))};
}

std::string row_json(const EvidenceRow& row) {
  std::ostringstream output;
  output << "{\"bytes\": " << row.bytes << ", \"path\": \"" << json_escape(row.path)
         << "\", \"sha256\": \"" << row.sha256 << "\"}";
  return output.str();
}

std::string raw_manifest_json(
    const Moment& session_started,
    const Moment& providers_enabled,
    const Moment& launcher_started,
    const Moment& launcher_exited,
    const Moment& session_stopped,
    DWORD launcher_process_id,
    DWORD launcher_exit_code,
    const EVENT_TRACE_PROPERTIES& statistics,
    const std::vector<ProviderMetadata>& providers,
    const EvidenceRow& etl,
    const EvidenceRow& events,
    const EvidenceRow& metadata,
    const EvidenceRow& transcript,
    std::string_view observer_sha256) {
  std::ostringstream output;
  output << "{\n"
         << "  \"candidateCommit\": \"" << utf8(kCandidateCommit) << "\",\n"
         << "  \"candidateImageSha256\": \"" << utf8(kCandidateImageSha256) << "\",\n"
         << "  \"candidateManifestSha256\": \"" << utf8(kCandidateManifestSha256) << "\",\n"
         << "  \"candidateTree\": \"" << utf8(kCandidateTree) << "\",\n"
         << "  \"clockType\": \"SYSTEM_TIME\",\n"
         << "  \"evidenceKind\": \"WINDOWS_PHASE2_ETW_RAW_OBSERVER_RUN\",\n"
         << "  \"files\": {\n"
         << "    \"etl\": " << row_json(etl) << ",\n"
         << "    \"events\": " << row_json(events) << ",\n"
         << "    \"metadata\": " << row_json(metadata) << ",\n"
         << "    \"transcript\": " << row_json(transcript) << "\n  },\n"
         << "  \"interval\": {\n"
         << "    \"launcherExitedAtFileTime\": \"" << launcher_exited.file_time << "\",\n"
         << "    \"launcherExitedAtUtc\": \"" << launcher_exited.utc << "\",\n"
         << "    \"launcherStartedAtFileTime\": \"" << launcher_started.file_time << "\",\n"
         << "    \"launcherStartedAtUtc\": \"" << launcher_started.utc << "\",\n"
         << "    \"providersEnabledAtFileTime\": \"" << providers_enabled.file_time << "\",\n"
         << "    \"providersEnabledAtUtc\": \"" << providers_enabled.utc << "\",\n"
         << "    \"startedAtFileTime\": \"" << session_started.file_time << "\",\n"
         << "    \"startedAtUtc\": \"" << session_started.utc << "\",\n"
         << "    \"stoppedAtFileTime\": \"" << session_stopped.file_time << "\",\n"
         << "    \"stoppedAtUtc\": \"" << session_stopped.utc << "\"\n  },\n"
         << "  \"launcher\": {\"exitCode\": " << launcher_exit_code
         << ", \"processId\": " << launcher_process_id << "},\n"
         << "  \"launcherSha256\": \"" << utf8(kLauncherSha256) << "\",\n"
         << "  \"observerBuildScriptSha256\": \"" << utf8(kObserverBuildScriptSha256) << "\",\n"
         << "  \"observerExecutableSha256\": \"" << observer_sha256 << "\",\n"
         << "  \"observerSourceSha256\": \"" << utf8(kObserverSourceSha256) << "\",\n"
         << "  \"observerVersion\": \"" << utf8(kObserverVersion) << "\",\n"
         << "  \"providers\": [\n";
  for (std::size_t index = 0; index < providers.size(); ++index) {
    const auto& provider = providers[index];
    output << "    {\"enableStatus\": " << provider.enable_status
           << ", \"level\": 5, \"matchAllKeyword\": \"0x0000000000000000\""
           << ", \"matchAnyKeyword\": \"" << hex_u64(provider.definition->match_any_keyword) << "\""
           << ", \"providerId\": \"" << utf8(guid_text(*provider.definition->id)) << "\""
           << ", \"registered\": true, \"role\": \"" << utf8(provider.definition->role) << "\"}"
           << (index + 1 == providers.size() ? "\n" : ",\n");
  }
  output << "  ],\n"
         << "  \"result\": \"RAW_CAPTURE_COMPLETE\",\n"
         << "  \"schemaVersion\": \"1.0\",\n"
         << "  \"sessionId\": \"" << utf8(guid_text(kSessionGuid)) << "\",\n"
         << "  \"sessionName\": \"" << utf8(kSessionName) << "\",\n"
         << "  \"traceStatistics\": {\"buffersWritten\": " << statistics.BuffersWritten
         << ", \"eventsLost\": " << statistics.EventsLost
         << ", \"logBuffersLost\": " << statistics.LogBuffersLost
         << ", \"realTimeBuffersLost\": " << statistics.RealTimeBuffersLost << "}\n"
         << "}\n";
  return output.str();
}

void verify_build_bindings(const std::filesystem::path& native_build,
                           const std::filesystem::path& root) {
  const auto package = native_build / L"package";
  const std::array<std::pair<std::filesystem::path, std::wstring_view>, 5> files{{
      {package / L"candidate-package-manifest.json", kCandidateManifestSha256},
      {package / L"GovsPLC.exe", kCandidateImageSha256},
      {native_build / L"Run-Native-E2E.exe", kLauncherSha256},
      {root / L"tools" / L"phase2" / L"windows_external_observer.cpp", kObserverSourceSha256},
      {root / L"tools" / L"phase2" / L"build_external_observer.mjs", kObserverBuildScriptSha256},
  }};
  for (const auto& [file, expected] : files) {
    if (sha256_file(file) != expected) fail("A fixed external-observer build input hash drifted.");
  }
}

int run() {
  const auto observer = executable_path();
  const auto native_build = observer.parent_path();
  const auto verification_root = native_build.parent_path();
  const auto root = verification_root.parent_path();
  verify_build_bindings(native_build, root);
  UniqueHandle singleton(CreateMutexW(
      nullptr, TRUE, L"Local\\GovsPLC-Phase2-External-Observer-Run-v1"));
  if (!singleton.valid()) fail("The fixed external-observer run lock is unavailable.");
  if (GetLastError() == ERROR_ALREADY_EXISTS) {
    fail("Another fixed external-observer run is still active.");
  }
  const auto evidence_root = verification_root / L"native-e2e";
  std::error_code error;
  std::filesystem::create_directories(evidence_root, error);
  if (error) fail("The fixed native evidence directory is unavailable.");
  const auto etl_path = evidence_root / L"native-gap-free-external-events.etl";
  const auto events_path = evidence_root / L"native-gap-free-external-events.jsonl";
  const auto metadata_path = evidence_root / L"native-gap-free-external-provider-metadata.json";
  const auto transcript_path = evidence_root / L"native-gap-free-external-observer-transcript.log";
  const auto raw_path = evidence_root / L"native-gap-free-external-observer-raw.json";
  const auto analysis_path = evidence_root / L"native-gap-free-external-observer-analysis.json";
  for (const auto& file : {etl_path, events_path, metadata_path, transcript_path, raw_path, analysis_path}) {
    delete_fixed_file(file);
  }
  std::vector<ProviderMetadata> providers;
  providers.reserve(kProviders.size());
  for (const auto& definition : kProviders) providers.push_back(provider_metadata(definition));
  ConsumerContext context;
  context.event_path = events_path;
  const Moment session_started = now();
  TraceSession trace(etl_path, context);
  trace.start_consumer();
  trace.enable(providers);
  const Moment providers_enabled = now();
  Moment launcher_started{};
  Moment launcher_exited{};
  const auto launcher_path = native_build / L"Run-Native-E2E.exe";
  const DWORD launcher_exit = run_fixed_launcher(launcher_path, context, launcher_started, launcher_exited);
  const DWORD launcher_pid = context.launcher_process_id.load();
  const EVENT_TRACE_PROPERTIES statistics = trace.stop();
  const Moment session_stopped = now();
  for (auto& provider : providers) {
    const std::wstring id = guid_text(*provider.definition->id);
    provider.observed_events = context.provider_counts.contains(id) ? context.provider_counts.at(id) : 0;
  }
  write_new_file(metadata_path, provider_metadata_json(providers, context));
  std::ostringstream transcript;
  transcript << "observerVersion=" << utf8(kObserverVersion) << '\n'
             << "candidateCommit=" << utf8(kCandidateCommit) << '\n'
             << "candidateTree=" << utf8(kCandidateTree) << '\n'
             << "sessionStartedAtUtc=" << session_started.utc << '\n'
             << "providersEnabledAtUtc=" << providers_enabled.utc << '\n'
             << "fixedLauncherProcessId=" << launcher_pid << '\n'
             << "fixedLauncherExitCode=" << launcher_exit << '\n'
             << "fixedLauncherExitedAtUtc=" << launcher_exited.utc << '\n'
             << "sessionStoppedAtUtc=" << session_stopped.utc << '\n'
             << "eventsLost=" << statistics.EventsLost << '\n'
             << "logBuffersLost=" << statistics.LogBuffersLost << '\n'
             << "realTimeBuffersLost=" << statistics.RealTimeBuffersLost << '\n';
  write_new_file(transcript_path, transcript.str());
  const auto etl = evidence_row(evidence_root, "native-gap-free-external-events.etl");
  const auto events = evidence_row(evidence_root, "native-gap-free-external-events.jsonl");
  const auto metadata = evidence_row(evidence_root, "native-gap-free-external-provider-metadata.json");
  const auto transcript_row = evidence_row(evidence_root, "native-gap-free-external-observer-transcript.log");
  const std::string observer_sha256 = utf8(sha256_file(observer));
  write_new_file(raw_path, raw_manifest_json(
      session_started, providers_enabled, launcher_started, launcher_exited, session_stopped,
      launcher_pid, launcher_exit, statistics, providers, etl, events, metadata, transcript_row,
      observer_sha256));
  if (launcher_exit != 0) {
    fail("The fixed exact-candidate launcher failed after its trace was preserved; exitCode=" +
         std::to_string(launcher_exit) + ".");
  }
  return 0;
}

}  // namespace

int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR command_line, int) {
  try {
    if (command_line == nullptr || std::wstring_view(command_line).find_first_not_of(L" \t\r\n") !=
                                       std::wstring_view::npos) {
      fail("The Phase 2 external observer accepts zero arguments and launches only its fixed exact candidate.");
    }
    return run();
  } catch (const std::exception& error) {
    try {
      const auto diagnostic = executable_path().parent_path() / L"external-observer-last-error.log";
      delete_fixed_file(diagnostic);
      write_new_file(diagnostic, std::string(error.what()) + "\n");
    } catch (...) {
      // The fixed diagnostic stream is best-effort only; never hide the failure.
    }
    MessageBoxA(nullptr, error.what(), "Gov's PLC Phase 2 external observer", MB_OK | MB_ICONERROR);
    return 1;
  }
}
