#pragma once

#include <windows.h>

#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <stdexcept>
#include <string>
#include <vector>

namespace govs::shell {

constexpr std::size_t kMaxProjectBytes = 32U * 1024U * 1024U;

enum class BrokerErrorCode : std::uint16_t {
  access_unavailable = 1,
  attestation_failed = 2,
  invalid_extension = 3,
  invalid_file_name = 4,
  invalid_frame = 5,
  project_too_large = 6,
  protocol_mismatch = 7,
  read_failed = 8,
  stale_grant = 9,
  unknown_grant = 10,
  write_failed = 11,
};

class BrokerFailure final : public std::runtime_error {
 public:
  BrokerFailure(BrokerErrorCode code, const char* message);
  [[nodiscard]] BrokerErrorCode code() const noexcept;

 private:
  BrokerErrorCode code_;
};

struct BrokerAttestation final {
  std::uint16_t protocol_version{};
  std::uint8_t file_system{};
  std::uint64_t volume_serial{};
  bool fixed_drive{};
  bool native_local{};
  bool provider_backed{};
  bool redirected{};
  bool removable{};
  bool special{};
};

struct OpenedProject final {
  std::string display_name;
  std::uint64_t grant_id{};
  std::vector<std::uint8_t> bytes;
};

struct SavedProject final {
  std::string display_name;
  std::uint64_t grant_id{};
  std::uint64_t verified_bytes{};
};

class BrokerClient final {
 public:
  BrokerClient() = default;
  BrokerClient(const BrokerClient&) = delete;
  BrokerClient& operator=(const BrokerClient&) = delete;
  ~BrokerClient();

  BrokerAttestation start();
  [[nodiscard]] std::filesystem::path prepare_fixed_user_data_folder(
      std::uint64_t session_id);
  void stop() noexcept;

  [[nodiscard]] std::vector<std::string> list_projects();
  [[nodiscard]] OpenedProject open(const std::string& name);
  [[nodiscard]] SavedProject save_as(
      const std::string& name,
      const std::vector<std::uint8_t>& bytes);
  [[nodiscard]] SavedProject save(
      std::uint64_t grant_id,
      const std::vector<std::uint8_t>& bytes);
  void revoke(std::uint64_t grant_id);

 private:
  struct Response;

  [[nodiscard]] Response transact(
      std::uint8_t operation,
      const std::vector<std::uint8_t>& payload);

  HANDLE input_{INVALID_HANDLE_VALUE};
  HANDLE output_{INVALID_HANDLE_VALUE};
  HANDLE process_{INVALID_HANDLE_VALUE};
  HANDLE job_{INVALID_HANDLE_VALUE};
  std::vector<HANDLE> authority_handles_;
  std::uint64_t next_request_id_{1};
};

}  // namespace govs::shell
