export type IsolationFuzzCase = Readonly<{
  category: string;
  id: string;
  value: string;
}>;

export const DEFAULT_FUZZ_CASES: readonly IsolationFuzzCase[];
export const FUZZ_CASE_IDS_SHA256: string;
export const FUZZ_CORPUS_SHA256: string;
export const ISOLATION_APPROVAL_DECISION_ID: string;
export const ISOLATION_APPROVAL_PATH: string;
export const REQUIRED_FUZZ_BOUNDARY_IDS: readonly string[];
export const SUPPORTED_CHROMIUM_RUNTIME_PRODUCTS: readonly string[];
export const SUPPORTED_WINDOWS_CONFIGURATION_IDS: readonly string[];
