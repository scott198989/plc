#pragma once

#include "broker_client.h"

#include <cstdint>
#include <string>
#include <vector>

namespace govs::shell {

enum class BridgeOperation { open, save_as, save, revoke };

struct BridgeRequest final {
  std::uint64_t request_id{};
  BridgeOperation operation{};
  std::string project_name;
  std::uint64_t grant_id{};
  std::vector<std::uint8_t> bytes;
};

[[nodiscard]] BridgeRequest parse_bridge_request(const std::wstring& message);
[[nodiscard]] std::wstring opened_bridge_response(
    std::uint64_t request_id,
    const OpenedProject& project);
[[nodiscard]] std::wstring saved_bridge_response(
    std::uint64_t request_id,
    const SavedProject& project);
[[nodiscard]] std::wstring revoked_bridge_response(std::uint64_t request_id);
[[nodiscard]] std::wstring error_bridge_response(
    std::uint64_t request_id,
    BrokerErrorCode code);
[[nodiscard]] std::wstring bridge_bootstrap_script(
    std::uint64_t volume_serial,
    std::uint64_t session_id,
    bool verification_mode);

}  // namespace govs::shell
