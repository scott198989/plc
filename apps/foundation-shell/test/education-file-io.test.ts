import { describe, expect, it } from "vitest";

import {
  ASSIGNMENT_FILE_EXTENSION,
  BUILT_IN_MOTOR_STARTER_ASSIGNMENT,
  SUBMISSION_FILE_EXTENSION,
} from "../src/education-contract";
import {
  EducationFileError,
  base64ToBytes,
  bytesToBase64,
  educationFileName,
  encodeEducationDocument,
  parseEducationDocumentText,
  verifyProjectArtifact,
} from "../src/education-file-io";

describe("offline education file helpers", () => {
  it("round-trips an assignment through deterministic local JSON bytes", () => {
    const bytes = encodeEducationDocument(BUILT_IN_MOTOR_STARTER_ASSIGNMENT);
    const text = new TextDecoder().decode(bytes);
    const parsed = parseEducationDocumentText(text, "assignment");

    expect(parsed).toEqual(BUILT_IN_MOTOR_STARTER_ASSIGNMENT);
    expect(text.indexOf('"assignmentId"')).toBeLessThan(text.indexOf('"title"'));
    expect(text).not.toContain("http://");
    expect(text).not.toContain("https://");
  });

  it("round-trips binary project packages without a network or text encoding", () => {
    const bytes = Uint8Array.from([0, 1, 2, 127, 128, 254, 255]);
    expect([...base64ToBytes(bytesToBase64(bytes))]).toEqual([...bytes]);
  });

  it("independently verifies the uppercase SHA-256 before opening an embedded project", async () => {
    const bytes = new TextEncoder().encode("canonical student project bytes");
    const sha256Hex = await hash(bytes);
    const verified = await verifyProjectArtifact({
      fileName: "Motor Starter.vlabproj",
      packageBase64: bytesToBase64(bytes),
      sha256Hex,
    });

    expect([...verified.bytes]).toEqual([...bytes]);
    expect(verified.sha256Hex).toBe(sha256Hex);
    await expect(verifyProjectArtifact({
      fileName: "Motor Starter.vlabproj",
      packageBase64: bytesToBase64(Uint8Array.from([...bytes, 1])),
      sha256Hex,
    })).rejects.toMatchObject({
      code: "ARTIFACT_HASH_MISMATCH",
    });
  });

  it("rejects unsafe names and non-uppercase or non-canonical artifact metadata", async () => {
    await expect(verifyProjectArtifact({
      fileName: "../student.vlabproj",
      packageBase64: "e30=",
      sha256Hex: "A".repeat(64),
    })).rejects.toMatchObject({
      code: "INVALID_ARTIFACT_FILE_NAME",
    });
    await expect(verifyProjectArtifact({
      fileName: "Student.vlabproj",
      packageBase64: "e30=",
      sha256Hex: "a".repeat(64),
    })).rejects.toMatchObject({
      code: "INVALID_ARTIFACT_HASH",
    });
    await expect(verifyProjectArtifact({
      fileName: "Student.vlabproj",
      packageBase64: "e30=\n",
      sha256Hex: "A".repeat(64),
    })).rejects.toMatchObject({
      code: "INVALID_ARTIFACT_BASE64",
    });
  });

  it("creates bounded safe assignment and submission download names", () => {
    expect(educationFileName("Motor: Start/Stop Lab", ASSIGNMENT_FILE_EXTENSION)).toBe(
      "Motor Start Stop Lab.vlabassign",
    );
    expect(educationFileName("<>|", SUBMISSION_FILE_EXTENSION)).toBe("Submission.vlabsubmit");
  });

  it("distinguishes malformed JSON from the wrong education schema", () => {
    expect(fileErrorCode(() => parseEducationDocumentText("{", "assignment"))).toBe("INVALID_JSON");
    expect(fileErrorCode(() => parseEducationDocumentText(
      JSON.stringify(BUILT_IN_MOTOR_STARTER_ASSIGNMENT),
      "submission",
    ))).toBe("INVALID_SCHEMA");
  });
});

const fileErrorCode = (operation: () => unknown): EducationFileError["code"] | null => {
  try {
    operation();
    return null;
  } catch (reason) {
    return reason instanceof EducationFileError ? reason.code : null;
  }
};

const hash = async (bytes: Uint8Array<ArrayBuffer>): Promise<string> => {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return [...digest]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("")
    .toUpperCase();
};
