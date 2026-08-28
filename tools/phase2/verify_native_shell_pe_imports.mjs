const forbidden = ["dnsapi.dll", "iphlpapi.dll", "urlmon.dll", "winhttp.dll", "wininet.dll", "ws2_32.dll"];

export function validateNativeShellImportText(text) {
  const normalized = text.toLocaleLowerCase("en-US");
  const found = forbidden.filter((name) => normalized.includes(name));
  if (found.length > 0) throw new Error(`Native shell has forbidden PE imports: ${found.join(", ")}`);
  if (!normalized.includes("kernel32.dll")) {
    throw new Error("Native shell PE import inventory is incomplete.");
  }
  return true;
}
