const REQUIRED_SOURCE_TOKENS = Object.freeze([
  "StartTraceW(&handle_",
  "EnableTraceEx2(",
  "ControlTraceW(handle_, kSessionName",
  "EVENT_TRACE_CONTROL_STOP",
  "PROCESS_TRACE_MODE_REAL_TIME",
  "PROCESS_TRACE_MODE_RAW_TIMESTAMP",
  "EVENT_TRACE_SYSTEM_LOGGER_MODE",
  "EVENT_TRACE_NO_PER_PROCESSOR_BUFFERING",
  "EVENT_TRACE_TYPE_START",
  "EVENT_TRACE_TYPE_STOP",
  "record->EventHeader.EventDescriptor.Opcode",
  "TdhEnumerateProviders(",
  "TdhEnumerateManifestProviderEvents(",
  "TdhGetManifestEventInformation(",
  "TdhGetEventInformation(",
  "TdhGetProperty(",
  "L\"ClientPID\"",
  "L\"ObserverProcessIdSource\"",
  "little_endian_afd_process_id(",
  "record->EventHeader.EventDescriptor.Id == 1000",
  "afd_process_token(properties)",
  "pid && *pid != 0",
  "client_pid_property",
  "else if (!client_pid_property)",
  "afd_process_ids",
  "ambiguous_afd_process_ids",
  "forget_afd_process_id(",
  "kernel-process-pid",
  "kernel-network-pid",
  "dns-client-pid",
  "dns-client-header-fallback",
  "name-resolution-header",
  "afd-create-process-id",
  "afd-process-map",
  "TokenLinkedToken",
  "TokenElevationTypeFull",
  "TokenElevationTypeLimited",
  "TokenIntegrityLevel",
  "TokenIsAppContainer",
  "TokenUIAccess",
  "GetShellWindow()",
  "DuplicateTokenEx(",
  "CreateProcessWithTokenW(",
  "CreateProcessAsUserW(",
  "ERROR_ACCESS_DENIED",
  "AdjustTokenPrivileges(",
  "SE_ASSIGNPRIMARYTOKEN_NAME",
  "SE_INCREASE_QUOTA_NAME",
  "LOGON_WITH_PROFILE",
  "winsta0\\\\default",
  "~TraceSession() noexcept",
  "failed after its trace was preserved",
  "CREATE_SUSPENDED",
  "Run-Native-E2E.exe",
  "native-gap-free-external-events.etl",
  "native-gap-free-external-events.jsonl",
  "native-gap-free-external-provider-metadata.json",
  "native-gap-free-external-observer-transcript.log",
  "statistics.EventsLost",
  "statistics.LogBuffersLost",
  "statistics.RealTimeBuffersLost",
  "accepts zero arguments and launches only its fixed exact candidate",
]);

const REQUIRED_PROVIDER_COMPONENTS = Object.freeze([
  "0x1c95126e, 0x7eea, 0x49a9",
  "0x22fb2cd6, 0x0e7b, 0x422b",
  "0x55404e71, 0x4db9, 0x4deb",
  "0x7dd42a49, 0x5329, 0x4832",
  "0xe53c6823, 0x7bb8, 0x44bb",
]);

const FORBIDDEN_SOURCE = Object.freeze([
  /\b(?:ShellExecute|WinExec|CreateService|LoadLibrary|GetProcAddress|URLDownloadToFile|WinHttpOpen|InternetOpen|DnsQuery|WSAStartup|DeviceIoControl)\s*\(/u,
  /\b(?:socket|connect|sendto|recvfrom|bind|listen)\s*\(/u,
  /\b(?:system|_popen|popen)\s*\(/u,
  /\b(?:COM[0-9]+|modbus|profinet|ethernet\/ip|s7comm|opc\s*ua)\b/iu,
  /\b(?:https?|ftp|wss?):\/\//iu,
]);

const FORBIDDEN_BUILD_LIBRARY = /\b(?:ws2_32|winhttp|wininet|urlmon|dnsapi|iphlpapi|setupapi|bluetoothapis|hid|winusb)\.lib\b/iu;
const EXPECTED_LINK_LIBRARIES = "advapi32.lib bcrypt.lib tdh.lib user32.lib";

export function verifyExternalObserverSources({ analyzer, build, finalizer, source }) {
  if (![analyzer, build, finalizer, source].every((value) => typeof value === "string" && value.length > 0)) {
    throw new Error("External observer source verification inputs are incomplete.");
  }
  for (const token of REQUIRED_SOURCE_TOKENS) {
    if (!source.includes(token)) throw new Error(`External observer invariant missing: ${token}`);
  }
  for (const token of REQUIRED_PROVIDER_COMPONENTS) {
    if (!source.includes(token)) throw new Error(`External observer provider missing: ${token}`);
  }
  if (source.includes("EVENT_TRACE_USE_PAGED_MEMORY")) {
    throw new Error("External observer system logger must use nonpaged ETW buffers.");
  }
  if (!source.includes("could not start; win32=")) {
    throw new Error("External observer StartTrace failure must retain the Win32 status.");
  }
  if (source.includes("CreateProcessW(")) {
    throw new Error("External observer must not fall back to the elevated process token.");
  }
  if (source.includes("payload_pid.value_or(record->EventHeader.ProcessId)")) {
    throw new Error("External observer provider attribution must not use a generic ETW header fallback.");
  }
  for (const pattern of FORBIDDEN_SOURCE) {
    const match = pattern.exec(source);
    if (match) throw new Error(`External observer forbidden capability: ${match[0]}`);
  }
  if (!build.includes(EXPECTED_LINK_LIBRARIES) || FORBIDDEN_BUILD_LIBRARY.test(build)) {
    throw new Error("External observer link-library inventory is not capability-minimized.");
  }
  for (const token of [
    "process.argv.slice(2).length !== 0",
    "Run-Phase2-External-Observer.exe",
    "candidate-package-manifest.json",
    "Run-Native-E2E.exe",
    "windows_external_observer.cpp",
    "/Brepro",
    "/guard:cf",
    "/CETCOMPAT",
  ]) {
    if (!build.includes(token)) throw new Error(`External observer build invariant missing: ${token}`);
  }
  for (const token of [
    "process.argv.slice(2).length !== 0",
    "native-gap-free-external-observer-raw.json",
    "native-gap-free-external-observer-analysis.json",
    "2 * 1024 * 1024 * 1024",
    "flag: \"wx\"",
  ]) {
    if (!finalizer.includes(token)) throw new Error(`External observer finalizer invariant missing: ${token}`);
  }
  for (const token of [
    "eventsLost === 0",
    "logBuffersLost === 0",
    "realTimeBuffersLost === 0",
    "Candidate process ${row.processId} lacks a covered teardown event.",
    "ProcessSequenceNumber",
    "ParentProcessSequenceNumber",
    "ambiguous candidate process lifetime attribution",
    "unknownEvents.length === 0",
    "unsupported-resolver-event-schema",
    "unsupported-afd-event-schema",
    "unsupported-kernel-network-event-schema",
    "network-target-unavailable",
    "resolver-invocation",
    "non-loopback-network-attempt",
  ]) {
    if (!analyzer.includes(token)) throw new Error(`External observer analyzer invariant missing: ${token}`);
  }
  return true;
}

export function forbiddenExternalObserverImport(dumpbinText) {
  if (typeof dumpbinText !== "string" || dumpbinText.length === 0) return "missing-import-inventory";
  const match = /\b(?:WS2_32|WINHTTP|WININET|URLMON|DNSAPI|IPHLPAPI|SETUPAPI|BLUETOOTHAPIS|HID|WINUSB)\.dll\b/iu.exec(dumpbinText);
  return match?.[0] ?? null;
}
