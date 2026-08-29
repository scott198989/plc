#include "bridge_protocol.h"

#include <algorithm>
#include <array>
#include <charconv>
#include <limits>
#include <ranges>
#include <sstream>
#include <span>
#include <string_view>

namespace govs::shell {
namespace {

constexpr std::string_view kRequestPrefix = "P2WEBQ1";
constexpr std::string_view kResponsePrefix = "P2WEBR1";
constexpr std::string_view kBase64Alphabet =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

[[noreturn]] void invalid_bridge(const char* message) {
  throw BrokerFailure(BrokerErrorCode::invalid_frame, message);
}

std::string narrow_ascii(const std::wstring& value) {
  std::string output;
  output.reserve(value.size());
  for (const auto character : value) {
    if (character > 0x7f) {
      invalid_bridge("The renderer bridge message was not bounded ASCII.");
    }
    output.push_back(static_cast<char>(character));
  }
  return output;
}

std::wstring widen_ascii(std::string_view value) {
  return std::wstring(value.begin(), value.end());
}

std::vector<std::string_view> split(std::string_view value) {
  std::vector<std::string_view> fields;
  std::size_t offset = 0;
  while (true) {
    const auto next = value.find('|', offset);
    fields.push_back(value.substr(
        offset, next == std::string_view::npos ? next : next - offset));
    if (next == std::string_view::npos) {
      return fields;
    }
    offset = next + 1;
    if (fields.size() > 6) {
      invalid_bridge("The renderer bridge message exposed extra fields.");
    }
  }
}

std::uint64_t parse_hex_id(std::string_view value) {
  if (value.size() != 16 ||
      !std::ranges::all_of(value, [](unsigned char byte) {
        return (byte >= '0' && byte <= '9') || (byte >= 'a' && byte <= 'f');
      })) {
    invalid_bridge("The renderer bridge identity was malformed.");
  }
  std::uint64_t result = 0;
  const auto parsed = std::from_chars(
      value.data(), value.data() + value.size(), result, 16);
  if (parsed.ec != std::errc{} || parsed.ptr != value.data() + value.size() ||
      result == 0) {
    invalid_bridge("The renderer bridge identity failed closed.");
  }
  return result;
}

std::string hex_id(std::uint64_t value) {
  std::array<char, 17> output{};
  const auto result = std::to_chars(
      output.data(), output.data() + 16, value, 16);
  if (result.ec != std::errc{}) {
    invalid_bridge("The native bridge identity could not be encoded.");
  }
  const auto digits = static_cast<std::size_t>(result.ptr - output.data());
  std::string padded(16 - digits, '0');
  padded.append(output.data(), digits);
  return padded;
}

std::vector<std::uint8_t> decode_base64(
    std::string_view value,
    std::size_t maximum) {
  if (value.empty() || value.size() > ((maximum + 2U) / 3U) * 4U ||
      value.size() % 4 != 0) {
    invalid_bridge("The renderer bridge base64 bound failed closed.");
  }
  std::array<std::int16_t, 256> table{};
  table.fill(-1);
  for (std::size_t index = 0; index < kBase64Alphabet.size(); ++index) {
    table[static_cast<unsigned char>(kBase64Alphabet[index])] =
        static_cast<std::int16_t>(index);
  }
  std::vector<std::uint8_t> output;
  output.reserve((value.size() / 4U) * 3U);
  for (std::size_t offset = 0; offset < value.size(); offset += 4) {
    const bool final = offset + 4 == value.size();
    const std::int16_t a = table[static_cast<unsigned char>(value[offset])];
    const std::int16_t b = table[static_cast<unsigned char>(value[offset + 1])];
    const std::int16_t c = value[offset + 2] == '='
                               ? std::int16_t{-2}
                               : table[static_cast<unsigned char>(value[offset + 2])];
    const std::int16_t d = value[offset + 3] == '='
                               ? std::int16_t{-2}
                               : table[static_cast<unsigned char>(value[offset + 3])];
    if (a < 0 || b < 0 || c == -1 || d == -1 ||
        (c == -2 && d != -2) || (!final && (c == -2 || d == -2))) {
      invalid_bridge("The renderer bridge base64 alphabet failed closed.");
    }
    const auto packed = (static_cast<std::uint32_t>(a) << 18U) |
                        (static_cast<std::uint32_t>(b) << 12U) |
                        (static_cast<std::uint32_t>(
                             std::max<std::int16_t>(c, std::int16_t{0}))
                         << 6U) |
                        static_cast<std::uint32_t>(
                            std::max<std::int16_t>(d, std::int16_t{0}));
    output.push_back(static_cast<std::uint8_t>(packed >> 16U));
    if (c != -2) {
      output.push_back(static_cast<std::uint8_t>(packed >> 8U));
    }
    if (d != -2) {
      output.push_back(static_cast<std::uint8_t>(packed));
    }
  }
  if (output.empty() || output.size() > maximum) {
    invalid_bridge("The decoded renderer bridge payload was out of bounds.");
  }
  return output;
}

std::string encode_base64(std::span<const std::uint8_t> bytes) {
  std::string output;
  output.reserve(((bytes.size() + 2U) / 3U) * 4U);
  for (std::size_t offset = 0; offset < bytes.size(); offset += 3) {
    const auto remaining = bytes.size() - offset;
    const auto packed = (static_cast<std::uint32_t>(bytes[offset]) << 16U) |
                        (remaining > 1
                             ? static_cast<std::uint32_t>(bytes[offset + 1]) << 8U
                             : 0U) |
                        (remaining > 2 ? bytes[offset + 2] : 0U);
    output.push_back(kBase64Alphabet[(packed >> 18U) & 0x3fU]);
    output.push_back(kBase64Alphabet[(packed >> 12U) & 0x3fU]);
    output.push_back(
        remaining > 1 ? kBase64Alphabet[(packed >> 6U) & 0x3fU] : '=');
    output.push_back(remaining > 2 ? kBase64Alphabet[packed & 0x3fU] : '=');
  }
  return output;
}

std::string encode_name(const std::string& name) {
  return encode_base64(std::span(
      reinterpret_cast<const std::uint8_t*>(name.data()), name.size()));
}

std::string decode_name(std::string_view value) {
  const auto bytes = decode_base64(value, 255);
  if (!std::ranges::all_of(bytes, [](std::uint8_t byte) {
        return byte >= 0x20 && byte <= 0x7e;
      })) {
    invalid_bridge("The renderer bridge project name was not bounded ASCII.");
  }
  return std::string(bytes.begin(), bytes.end());
}

std::string error_code(BrokerErrorCode code) {
  switch (code) {
    case BrokerErrorCode::attestation_failed:
      return "ATTESTATION_FAILED";
    case BrokerErrorCode::stale_grant:
    case BrokerErrorCode::unknown_grant:
      return "STALE_GRANT";
    case BrokerErrorCode::invalid_extension:
    case BrokerErrorCode::invalid_file_name:
      return "INVALID_FILE_NAME";
    case BrokerErrorCode::project_too_large:
      return "PROJECT_TOO_LARGE";
    case BrokerErrorCode::write_failed:
      return "WRITE_FAILED";
    case BrokerErrorCode::read_failed:
      return "READ_FAILED";
    case BrokerErrorCode::access_unavailable:
    case BrokerErrorCode::invalid_frame:
    case BrokerErrorCode::protocol_mismatch:
    default:
      return "ACCESS_UNAVAILABLE";
  }
}

}  // namespace

BridgeRequest parse_bridge_request(const std::wstring& message) {
  if (message.size() > ((kMaxProjectBytes + 2U) / 3U) * 4U + 512U) {
    invalid_bridge("The renderer bridge message exceeded its closed bound.");
  }
  const auto ascii = narrow_ascii(message);
  const auto fields = split(ascii);
  if (fields.size() < 3 || fields[0] != kRequestPrefix) {
    invalid_bridge("The renderer bridge protocol prefix failed closed.");
  }
  BridgeRequest request{};
  request.request_id = parse_hex_id(fields[1]);
  if (fields[2] == "open" && fields.size() == 3) {
    request.operation = BridgeOperation::open;
  } else if (fields[2] == "save-as" && fields.size() == 5) {
    request.operation = BridgeOperation::save_as;
    request.project_name = decode_name(fields[3]);
    request.bytes = decode_base64(fields[4], kMaxProjectBytes);
  } else if (fields[2] == "save" && fields.size() == 5) {
    request.operation = BridgeOperation::save;
    request.grant_id = parse_hex_id(fields[3]);
    request.bytes = decode_base64(fields[4], kMaxProjectBytes);
  } else if (fields[2] == "revoke" && fields.size() == 4) {
    request.operation = BridgeOperation::revoke;
    request.grant_id = parse_hex_id(fields[3]);
  } else {
    invalid_bridge("The renderer bridge operation surface failed closed.");
  }
  return request;
}

std::wstring opened_bridge_response(
    std::uint64_t request_id,
    const OpenedProject& project) {
  const auto response = std::string(kResponsePrefix) + "|" + hex_id(request_id) +
                        "|opened|" + encode_name(project.display_name) + "|" +
                        hex_id(project.grant_id) + "|" + encode_base64(project.bytes);
  return widen_ascii(response);
}

std::wstring saved_bridge_response(
    std::uint64_t request_id,
    const SavedProject& project) {
  const auto response = std::string(kResponsePrefix) + "|" + hex_id(request_id) +
                        "|saved|" + encode_name(project.display_name) + "|" +
                        hex_id(project.grant_id) + "|" +
                        std::to_string(project.verified_bytes);
  return widen_ascii(response);
}

std::wstring revoked_bridge_response(std::uint64_t request_id) {
  return widen_ascii(
      std::string(kResponsePrefix) + "|" + hex_id(request_id) + "|revoked");
}

std::wstring error_bridge_response(
    std::uint64_t request_id,
    BrokerErrorCode code) {
  return widen_ascii(std::string(kResponsePrefix) + "|" + hex_id(request_id) +
                     "|error|" + error_code(code));
}

std::wstring bridge_bootstrap_script(
    std::uint64_t volume_serial,
    std::uint64_t session_id,
    bool verification_mode) {
  std::ostringstream attestation;
  attestation << std::uppercase << std::hex;
  attestation.width(8);
  attestation.fill('0');
  attestation << volume_serial;
  attestation << ':';
  attestation.width(16);
  attestation << session_id;

  std::string script = R"JS((() => {
  "use strict";
  if (window !== window.top) return;
  const channel = window.chrome && window.chrome.webview;
  if (!channel || typeof channel.postMessage !== "function" || typeof channel.addEventListener !== "function") return;
  const deniedBrowserFileCapability = Object.freeze(() => {
    throw Object.freeze({ code: "ACCESS_UNAVAILABLE" });
  });
  for (const name of ["print", "showOpenFilePicker", "showSaveFilePicker", "showDirectoryPicker"]) {
    Object.defineProperty(window, name, {
      configurable: false,
      enumerable: false,
      value: deniedBrowserFileCapability,
      writable: false,
    });
  }
  const prefix = "P2WEBQ1";
  const responsePrefix = "P2WEBR1";
  const attestationId = "fixed-local-v1:)JS" + attestation.str() + R"JS(";
  let nextId = 1n;
  const pending = new Map();
  let verificationGrant = null;
  let verificationResponses = 0;
  const verificationUuidVersion = "govs-p2-native-verification-uuid-v1";
  const verificationUuidSeed = "2B42B846-54D0-4C61-9B72-4CD3AFC50001";
  const verificationUuidOrdinalContract = "after-saved-document:build=4,power=5,preview=6,commit=7,online=8,run=9,scan=10,stop=11,capture=12";
  let verificationUuidSequence = 1n;
  if ()JS" + std::string(verification_mode ? "true" : "false") + R"JS() {
    const deterministicUuid = () => {
      const tail = verificationUuidSequence.toString(16).padStart(12, "0");
      verificationUuidSequence += 1n;
      if (verificationUuidSequence > 0xffffffffffffffffn) throw new Error("verification UUID sequence exhausted");
      return `2b42b846-54d0-4c61-9b72-${tail}`;
    };
    Object.defineProperty(globalThis.crypto, "randomUUID", {
      configurable: false,
      enumerable: false,
      value: deterministicUuid,
      writable: false,
    });
    Object.defineProperty(globalThis, "govsP2VerificationUuidV1", {
      configurable: false,
      enumerable: false,
      value: Object.freeze({ ordinalContract: verificationUuidOrdinalContract, seed: verificationUuidSeed, version: verificationUuidVersion }),
      writable: false,
    });
  }
  const hex = value => value.toString(16).padStart(16, "0");
  const bytesToBase64 = bytes => {
    if (!(bytes instanceof Uint8Array) || bytes.byteLength < 1 || bytes.byteLength > 33554432) throw Object.freeze({ code: "PROJECT_TOO_LARGE" });
    let binary = "";
    for (let offset = 0; offset < bytes.byteLength; offset += 32768) {
      binary += String.fromCharCode(...bytes.subarray(offset, Math.min(offset + 32768, bytes.byteLength)));
    }
    return btoa(binary);
  };
  const base64ToBytes = value => {
    const binary = atob(value);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
    return bytes;
  };
  const textToBase64 = value => bytesToBase64(new TextEncoder().encode(value));
  const base64ToText = value => new TextDecoder("utf-8", { fatal: true }).decode(base64ToBytes(value));
  const allocateId = () => {
    if (nextId > 0xffffffffffffffffn) throw Object.freeze({ code: "ACCESS_UNAVAILABLE" });
    const value = hex(nextId);
    nextId += 1n;
    return value;
  };
  const request = (operation, fields, acknowledgementTimeoutMilliseconds = 0) => new Promise((resolve, reject) => {
    if (pending.size >= 64) {
      reject(Object.freeze({ code: "ACCESS_UNAVAILABLE" }));
      return;
    }
    const id = allocateId();
    const timeout = acknowledgementTimeoutMilliseconds > 0
      ? setTimeout(() => {
          const target = pending.get(id);
          if (!target) return;
          pending.delete(id);
          target.reject(Object.freeze({ code: "ACCESS_UNAVAILABLE" }));
        }, acknowledgementTimeoutMilliseconds)
      : null;
    pending.set(id, Object.freeze({ reject, resolve, timeout }));
    try {
      channel.postMessage([prefix, id, operation, ...fields].join("|"));
    } catch {
      pending.delete(id);
      if (timeout !== null) clearTimeout(timeout);
      reject(Object.freeze({ code: "ACCESS_UNAVAILABLE" }));
    }
  });
  channel.addEventListener("message", event => {
    if (typeof event.data !== "string") return;
    const fields = event.data.split("|");
    if (fields.length < 3 || fields[0] !== responsePrefix || !/^[0-9a-f]{16}$/.test(fields[1])) return;
    const target = pending.get(fields[1]);
    if (!target) return;
    pending.delete(fields[1]);
    if (target.timeout !== null) clearTimeout(target.timeout);
    try {
      if (fields[2] === "error" && fields.length === 4 && /^[A-Z_]+$/.test(fields[3])) {
        target.reject(Object.freeze({ code: fields[3] }));
      } else if (fields[2] === "opened" && fields.length === 6 && /^[0-9a-f]{16}$/.test(fields[4])) {
        const value = Object.freeze({
          attestationId,
          bytes: base64ToBytes(fields[5]),
          displayName: base64ToText(fields[3]),
          grantId: `p2-native-v1:${fields[4]}`,
          protocolVersion: 1,
        });
        verificationGrant = value.grantId;
        verificationResponses += 1;
        target.resolve(value);
      } else if (fields[2] === "saved" && fields.length === 6 && /^[0-9a-f]{16}$/.test(fields[4]) && /^[0-9]+$/.test(fields[5])) {
        const verifiedBytes = Number(fields[5]);
        if (!Number.isSafeInteger(verifiedBytes) || verifiedBytes < 1 || verifiedBytes > 33554432) throw new Error("invalid verified length");
        const value = Object.freeze({
          attestationId,
          displayName: base64ToText(fields[3]),
          grantId: `p2-native-v1:${fields[4]}`,
          protocolVersion: 1,
          verifiedBytes,
        });
        verificationGrant = value.grantId;
        verificationResponses += 1;
        target.resolve(value);
      } else if (fields[2] === "revoked" && fields.length === 3) {
        target.resolve(undefined);
      } else {
        target.reject(Object.freeze({ code: "ACCESS_UNAVAILABLE" }));
      }
    } catch {
      target.reject(Object.freeze({ code: "ACCESS_UNAVAILABLE" }));
    }
  });
  window.addEventListener("pagehide", () => {
    for (const target of pending.values()) {
      if (target.timeout !== null) clearTimeout(target.timeout);
      target.reject(Object.freeze({ code: "ACCESS_UNAVAILABLE" }));
    }
    pending.clear();
  }, { once: true });
  const grantField = value => {
    if (typeof value !== "string" || !/^p2-native-v1:[0-9a-f]{16}$/.test(value)) throw Object.freeze({ code: "STALE_GRANT" });
    return value.slice("p2-native-v1:".length);
  };
  const attestation = Object.freeze({
    attestationId,
    fixedDrive: true,
    kind: "fixed-native-local-v1",
    nativeLocal: true,
    platform: "windows",
    providerBacked: false,
    redirected: false,
    removable: false,
    special: false,
  });
  const terminateAfterRevokeFailure = () => {
    try {
      channel.postMessage("P2REVKF1");
    } catch {
      // A dead renderer-to-host channel cannot retain a usable renderer grant.
      // Closing the top-level view is the last local fail-closed action; the
      // host also tears down the helper on process/view failure and shutdown.
      window.close();
    }
  };
  const bridge = Object.freeze({
    attestation,
    contract: "govs.project-file-broker",
    open: () => request("open", []),
    protocolVersion: 1,
    revoke: grantId => {
      void request("revoke", [grantField(grantId)], 5000).catch(terminateAfterRevokeFailure);
    },
    save: ({ bytes, grantId, protocolVersion }) => {
      if (protocolVersion !== 1) return Promise.reject(Object.freeze({ code: "ACCESS_UNAVAILABLE" }));
      return request("save", [grantField(grantId), bytesToBase64(bytes)]);
    },
    saveAs: ({ bytes, projectName, protocolVersion }) => {
      if (protocolVersion !== 1 || typeof projectName !== "string") return Promise.reject(Object.freeze({ code: "ACCESS_UNAVAILABLE" }));
      return request("save-as", [textToBase64(projectName), bytesToBase64(bytes)]);
    },
  });
  Object.defineProperty(window, "govsProjectFileBrokerV1", {
    configurable: false,
    enumerable: false,
    value: bridge,
    writable: false,
  });
)JS";
  if (verification_mode) {
    script += R"JS(
  window.addEventListener("DOMContentLoaded", async () => {
    try {
      const waitFor = async (predicate, label) => {
        const deadline = performance.now() + 30000;
        while (performance.now() < deadline) {
          const alert = document.querySelector('[role="alert"]');
          if (alert !== null) throw new Error(`verification UI alert during ${label}: ${alert.textContent?.trim() ?? "unknown"}`);
          const value = predicate();
          if (value) return value;
          await new Promise(resolve => setTimeout(resolve, 25));
        }
        throw new Error(`verification UI timeout: ${label}`);
      };
      const buttonWithText = text => [...document.querySelectorAll("button")]
        .find(button => {
          const directText = [...button.childNodes]
            .filter(node => node.nodeType === Node.TEXT_NODE)
            .map(node => node.textContent?.trim() ?? "")
            .filter(Boolean)
            .join(" ");
          return !button.disabled &&
            (button.getAttribute("aria-label")?.trim() === text || directText === text ||
              button.textContent?.trim() === text);
        });
      const settled = predicate => !document.querySelector(".status-segment--busy") && predicate();
      const cpuIs = state => document.querySelector(`.runtime-summary__state[data-state="${state}"]`) !== null;
      const scanSequenceIs = value => [...document.querySelectorAll(".runtime-summary dl > div")]
        .some(entry => entry.querySelector("dt")?.textContent?.trim() === "Scan sequence" &&
          entry.querySelector("dd")?.textContent?.trim() === String(value));
      const buildIsCurrent = () => [...document.querySelectorAll(".status-segment")]
        .some(segment => segment.textContent?.trim() === "Build current");
      const name = await waitFor(
        () => [...document.querySelectorAll("input")]
          .find(input => [...(input.labels ?? [])]
            .some(label => label.textContent?.trim() === "Project name") && !input.disabled),
        "new-project name",
      );
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
      if (typeof setter !== "function") throw new Error("input setter unavailable");
       setter.call(name, "Phase 2 Native Verification");
       name.dispatchEvent(new Event("input", { bubbles: true }));
       (await waitFor(() => buttonWithText("Create"), "create")).click();
       const selectedTreeItem = label => [...document.querySelectorAll('button[role="treeitem"]')]
         .find(button => button.getAttribute("aria-selected") === "true" &&
           button.querySelector(".tree-label")?.textContent?.trim() === label);
       const selectTreeItem = async label => {
         const item = await waitFor(() => [...document.querySelectorAll('button[role="treeitem"]')]
           .find(button => button.querySelector(".tree-label")?.textContent?.trim() === label), `tree item ${label}`);
         item.click();
         await waitFor(() => selectedTreeItem(label) && !document.querySelector(".status-segment--busy"), `selected ${label}`);
       };
       const createChild = async (label, expectedName) => {
         (await waitFor(() => {
           const button = document.querySelector('button[aria-label="Add engineering object"]');
           return button && !button.disabled ? button : null;
         }, `add ${label}`)).click();
         (await waitFor(() => [...document.querySelectorAll('[role="menuitem"]')]
           .find(button => button.querySelector("strong")?.textContent?.trim() === label && !button.disabled), `menu ${label}`)).click();
         await waitFor(() => selectedTreeItem(expectedName) && !document.querySelector(".status-segment--busy") &&
           document.querySelector(".diagnostics-summary") !== null, `created ${label} with diagnostics settled`);
       };
       await createChild("Virtual network", "Virtual network");
       await selectTreeItem("Phase 2 Native Verification");
       await createChild("Controller", "Controller");
       await createChild("Rack", "Local rack");
       await selectTreeItem("Local rack");
       await createChild("VDI16", "VDI16");
       await selectTreeItem("Local rack");
       await createChild("VDO16", "VDO16");
       await selectTreeItem("Controller");
       await createChild("Organization block", "Main_cycle");
       (await waitFor(
        () => [...document.querySelectorAll("button")]
          .find(button => button.getAttribute("title") === "Save as" && !button.disabled),
        "save as",
      )).click();
      await waitFor(() => verificationResponses >= 1, "native create response");
      (await waitFor(() => buttonWithText("Close"), "close after create")).click();
      (await waitFor(() => buttonWithText("Choose project file"), "open project")).click();
       await waitFor(() => verificationResponses >= 2, "native open response");
       (await waitFor(() => buttonWithText("Build"), "runtime build")).click();
       await waitFor(() => settled(buildIsCurrent), "Build current");
       (await waitFor(() => buttonWithText("Power on"), "runtime power on")).click();
       await waitFor(() => settled(() => cpuIs("STOP") && buttonWithText("Power off") !== undefined), "power on STOP state");
       (await waitFor(() => buttonWithText("Preview load"), "runtime load preview")).click();
       await waitFor(() => settled(() => buttonWithText("Commit load") !== undefined), "preview load ready to commit");
       (await waitFor(() => buttonWithText("Commit load"), "runtime commit load")).click();
       await waitFor(() => settled(() => buttonWithText("Go online") !== undefined), "committed load ready for online");
      (await waitFor(() => buttonWithText("Go online"), "runtime go online")).click();
      await waitFor(() => settled(() => buttonWithText("RUN") !== undefined), "online STOP state");
      (await waitFor(() => buttonWithText("RUN"), "runtime run")).click();
      await waitFor(() => settled(() => cpuIs("RUN") && buttonWithText("STOP") !== undefined), "RUN state");
      (await waitFor(() => buttonWithText("Scan +1"), "runtime scan")).click();
      await waitFor(() => settled(() => scanSequenceIs(1)), "scan sequence 1");
      (await waitFor(() => buttonWithText("STOP"), "runtime stop for replay snapshot")).click();
      await waitFor(() => settled(() => cpuIs("STOP")), "STOP state before capture");
      (await waitFor(() => buttonWithText("Capture snapshot"), "capture replay snapshot")).click();
      await waitFor(() => settled(() => buttonWithText("Verify replay") !== undefined), "captured replay snapshot");
      (await waitFor(() => buttonWithText("Verify replay"), "closed replay verification")).click();
      const verifiedReplay = await waitFor(() => {
        const receipt = document.querySelector('[aria-label="Replay verified"]');
        const summary = document.querySelector('[aria-label="Replay verification receipt"] span');
        const fingerprint = receipt?.getAttribute("title") ?? "";
        const toolbarMatch = /^Replay verified · ([1-9][0-9]*) events$/.exec(receipt?.textContent?.trim() ?? "");
        const summaryMatch = /^([1-9][0-9]*) events · ([1-9][0-9]*) boundary$/.exec(summary?.textContent?.trim() ?? "");
        return /^[0-9A-F]{64}$/.test(fingerprint) && toolbarMatch && summaryMatch &&
          toolbarMatch[1] === summaryMatch[1]
          ? Object.freeze({
              boundaryCount: summaryMatch[2],
              eventCount: summaryMatch[1],
              fingerprint,
            })
          : null;
      }, "verified replay receipt");
      const runtimeReplayHash = await waitFor(() => {
        const receipt = [...document.querySelectorAll(".runtime-toolbar__receipt")]
          .find(candidate => !candidate.hasAttribute("aria-label") && /^e[0-9]+ · s[0-9]+$/.test(candidate.textContent?.trim() ?? ""));
        const hash = receipt?.getAttribute("title") ?? "";
        return /^[0-9A-F]{64}$/.test(hash) ? hash : null;
      }, "canonical runtime replay");
      channel.postMessage(
        `P2VEFY1|${runtimeReplayHash}|${verifiedReplay.fingerprint}|${verifiedReplay.eventCount}|${verifiedReplay.boundaryCount}`,
      );
      (await waitFor(() => buttonWithText("Save"), "replace project")).click();
      await waitFor(() => verificationResponses >= 3, "native replace response");
      if (verificationGrant === null) throw new Error("verification grant unavailable");
      bridge.revoke(verificationGrant);
    } catch (error) {
      const detail = String(error instanceof Error ? error.message : "verification UI failure")
        .replace(/[^A-Za-z0-9 .:_-]/g, "?")
        .slice(0, 512);
      try { channel.postMessage(`P2VEFY0|${detail || "verification UI failure"}`); } catch { /* host timeout remains fail-closed */ }
    }
  }, { once: true });
)JS";
  }
  script += "\n})();";
  return std::wstring(script.begin(), script.end());
}

}  // namespace govs::shell
