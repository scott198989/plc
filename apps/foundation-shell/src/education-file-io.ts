import { encodeCanonicalJson } from "./canonical-json";
import {
  ASSIGNMENT_FILE_EXTENSION,
  SUBMISSION_FILE_EXTENSION,
  parseAssignmentDocument,
  parseSubmissionDocument,
} from "./education-contract";
import type {
  AssignmentDocumentV1,
  ProjectArtifactV1,
  SubmissionDocumentV1,
} from "./education-contract";

const MAX_EDUCATION_FILE_BYTES = 48 * 1024 * 1024;
const MAX_PROJECT_ARTIFACT_BYTES = 32 * 1024 * 1024;
const MAX_PROJECT_ARTIFACT_BASE64_CHARACTERS = Math.ceil(MAX_PROJECT_ARTIFACT_BYTES / 3) * 4;
const PROJECT_ARTIFACT_FILE_EXTENSION = ".vlabproj";
const CANONICAL_BASE64 = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/u;
const UPPER_SHA256 = /^[A-F0-9]{64}$/u;
const UNSAFE_PROJECT_FILE_NAME = /[\\/:*?"<>|\u0000-\u001f\u007f]/u;

export type EducationDocument = AssignmentDocumentV1 | SubmissionDocumentV1;
export type EducationDocumentKind = "assignment" | "submission";

export class EducationFileError extends Error {
  public readonly code: "FILE_EMPTY" | "FILE_TOO_LARGE" | "INVALID_EXTENSION" | "INVALID_JSON" | "INVALID_SCHEMA";

  public constructor(code: EducationFileError["code"], message: string) {
    super(message);
    this.name = "EducationFileError";
    this.code = code;
  }
}

export class ProjectArtifactVerificationError extends Error {
  public readonly code:
    | "ARTIFACT_HASH_MISMATCH"
    | "INVALID_ARTIFACT_BASE64"
    | "INVALID_ARTIFACT_FILE_NAME"
    | "INVALID_ARTIFACT_HASH"
    | "INVALID_ARTIFACT_SIZE";

  public constructor(code: ProjectArtifactVerificationError["code"], message: string) {
    super(message);
    this.name = "ProjectArtifactVerificationError";
    this.code = code;
  }
}

export type VerifiedProjectArtifact = Readonly<{
  bytes: Uint8Array<ArrayBuffer>;
  fileName: string;
  sha256Hex: string;
}>;

export const encodeEducationDocument = (document: EducationDocument): Uint8Array<ArrayBuffer> =>
  encodeCanonicalJson(document);

/** Converts project-package bytes without spreading a multi-megabyte buffer onto the call stack. */
export const bytesToBase64 = (bytes: Uint8Array<ArrayBuffer>): string => {
  const fragments: string[] = [];
  const chunkSize = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    fragments.push(String.fromCharCode(...bytes.subarray(offset, offset + chunkSize)));
  }
  return btoa(fragments.join(""));
};

export const base64ToBytes = (value: string): Uint8Array<ArrayBuffer> => {
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
};

/**
 * Revalidates an embedded project at the point where student-authored bytes become executable
 * engineering input. The education document parser validates shape; this function independently
 * proves the decoded bytes match the declared digest before they cross the worker boundary.
 */
export const verifyProjectArtifact = async (
  artifact: ProjectArtifactV1,
): Promise<VerifiedProjectArtifact> => {
  if (typeof artifact.fileName !== "string" || !isSafeProjectArtifactFileName(artifact.fileName)) {
    throw new ProjectArtifactVerificationError(
      "INVALID_ARTIFACT_FILE_NAME",
      "The submitted project must have one safe .vlabproj file name.",
    );
  }
  if (typeof artifact.sha256Hex !== "string" || !UPPER_SHA256.test(artifact.sha256Hex)) {
    throw new ProjectArtifactVerificationError(
      "INVALID_ARTIFACT_HASH",
      "The submitted project digest must be an uppercase SHA-256 value.",
    );
  }
  if (
    typeof artifact.packageBase64 !== "string"
    || artifact.packageBase64.length < 4
    || artifact.packageBase64.length > MAX_PROJECT_ARTIFACT_BASE64_CHARACTERS
    || !CANONICAL_BASE64.test(artifact.packageBase64)
  ) {
    throw new ProjectArtifactVerificationError(
      "INVALID_ARTIFACT_BASE64",
      "The submitted project package is not canonical Base64.",
    );
  }

  const decodedSize = decodedCanonicalBase64Size(artifact.packageBase64);
  if (decodedSize < 1 || decodedSize > MAX_PROJECT_ARTIFACT_BYTES) {
    throw new ProjectArtifactVerificationError(
      "INVALID_ARTIFACT_SIZE",
      "The submitted project package violates the 32 MiB project limit.",
    );
  }

  let bytes: Uint8Array<ArrayBuffer>;
  try {
    bytes = base64ToBytes(artifact.packageBase64);
  } catch {
    throw new ProjectArtifactVerificationError(
      "INVALID_ARTIFACT_BASE64",
      "The submitted project package could not be decoded.",
    );
  }
  if (bytes.byteLength !== decodedSize) {
    throw new ProjectArtifactVerificationError(
      "INVALID_ARTIFACT_SIZE",
      "The submitted project package decoded to an unexpected size.",
    );
  }

  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  const actualHash = [...digest]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("")
    .toUpperCase();
  if (actualHash !== artifact.sha256Hex) {
    throw new ProjectArtifactVerificationError(
      "ARTIFACT_HASH_MISMATCH",
      "The submitted project bytes do not match the digest recorded in the submission.",
    );
  }
  return { bytes, fileName: artifact.fileName, sha256Hex: actualHash };
};

const decodedCanonicalBase64Size = (value: string): number => {
  const padding = value.endsWith("==") ? 2 : value.endsWith("=") ? 1 : 0;
  return (value.length / 4) * 3 - padding;
};

const isSafeProjectArtifactFileName = (value: string): boolean =>
  value.length > 0
  && value.length <= 255
  && value === value.trim()
  && value.toLocaleLowerCase("en-US").endsWith(PROJECT_ARTIFACT_FILE_EXTENSION)
  && !UNSAFE_PROJECT_FILE_NAME.test(value);

export function parseEducationDocumentText(
  text: string,
  kind: "assignment",
): AssignmentDocumentV1;
export function parseEducationDocumentText(
  text: string,
  kind: "submission",
): SubmissionDocumentV1;
export function parseEducationDocumentText(
  text: string,
  kind: EducationDocumentKind,
): EducationDocument {
  if (text.length === 0) throw new EducationFileError("FILE_EMPTY", "The selected file is empty.");
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new EducationFileError("INVALID_JSON", "The selected file does not contain valid JSON.");
  }
  try {
    return kind === "assignment" ? parseAssignmentDocument(value) : parseSubmissionDocument(value);
  } catch {
    throw new EducationFileError(
      "INVALID_SCHEMA",
      kind === "assignment"
        ? "The selected file is not a supported virtual-lab assignment."
        : "The selected file is not a supported virtual-lab submission.",
    );
  }
}

export function readEducationFile(
  file: File,
  kind: "assignment",
): Promise<AssignmentDocumentV1>;
export function readEducationFile(
  file: File,
  kind: "submission",
): Promise<SubmissionDocumentV1>;
export async function readEducationFile(
  file: File,
  kind: EducationDocumentKind,
): Promise<EducationDocument> {
  const extension = kind === "assignment" ? ASSIGNMENT_FILE_EXTENSION : SUBMISSION_FILE_EXTENSION;
  if (!file.name.toLocaleLowerCase("en-US").endsWith(extension)) {
    throw new EducationFileError("INVALID_EXTENSION", `Choose a ${extension} file.`);
  }
  if (file.size < 1) throw new EducationFileError("FILE_EMPTY", "The selected file is empty.");
  if (file.size > MAX_EDUCATION_FILE_BYTES) {
    throw new EducationFileError("FILE_TOO_LARGE", "The selected education file is too large.");
  }
  const text = await file.text();
  return kind === "assignment"
    ? parseEducationDocumentText(text, "assignment")
    : parseEducationDocumentText(text, "submission");
}

export const educationFileName = (
  label: string,
  extension: typeof ASSIGNMENT_FILE_EXTENSION | typeof SUBMISSION_FILE_EXTENSION,
): string => {
  const normalized = label
    .replace(/[<>:"/\\|?*\u0000-\u001f\u007f]/gu, " ")
    .replace(/[^A-Za-z0-9 _().-]/gu, " ")
    .replace(/\s+/gu, " ")
    .trim()
    .replace(/[. ]+$/u, "")
    .slice(0, 180)
    .replace(/[. ]+$/u, "");
  return `${normalized || (extension === ASSIGNMENT_FILE_EXTENSION ? "Assignment" : "Submission")}${extension}`;
};

/** Uses a browser download only; no request, endpoint, or network API is involved. */
export const downloadEducationDocument = (
  document: EducationDocument,
  fileName: string,
): void => {
  const expectedExtension = document.documentKind === "vlab-assignment"
    ? ASSIGNMENT_FILE_EXTENSION
    : SUBMISSION_FILE_EXTENSION;
  const safeName = educationFileName(
    fileName.toLocaleLowerCase("en-US").endsWith(expectedExtension)
      ? fileName.slice(0, -expectedExtension.length)
      : fileName,
    expectedExtension,
  );
  const blob = new Blob([encodeEducationDocument(document)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = window.document.createElement("a");
  anchor.download = safeName;
  anchor.href = url;
  anchor.hidden = true;
  window.document.body.append(anchor);
  anchor.click();
  anchor.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
};
