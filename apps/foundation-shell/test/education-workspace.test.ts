import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import {
  EducationWorkspace,
  isEducatorModeLocked,
  SubmissionReviewCommentEditor,
  TeacherAssignmentStudio,
} from "../src/EducationWorkspace";
import { createBlankAssignmentDraft } from "../src/education-workflow";

describe("education workspace presentation", () => {
  it("locks educator tools while a student assignment is active", () => {
    expect(isEducatorModeLocked("student", true)).toBe(true);
    expect(isEducatorModeLocked("student", false)).toBe(false);
    expect(isEducatorModeLocked("teacher", true)).toBe(false);
  });

  it("opens with a clear student assignment choice and honest offline controls", () => {
    const markup = renderToStaticMarkup(createElement(EducationWorkspace, {
      busy: false,
      snapshot: {
        buildState: "not-built",
        diagnostics: [],
        objects: {},
        projectHash: "A".repeat(64),
        projectName: "Motor Lab",
        runtime: {
          availability: "UNAVAILABLE",
          canBuild: false,
          diagnostics: [],
          reason: "Configure a virtual PLC.",
          schemaVersion: 1,
          session: null,
          sourceDocumentHash: "A".repeat(64),
          sourceSemanticFingerprint: "B".repeat(64),
        },
      },
    }));

    expect(markup).toContain('aria-label="Education workspace"');
    expect(markup).toContain("Student Mission");
    expect(markup).toContain("Your first motor starter");
    expect(markup).toContain("Open mission");
    expect(markup).toContain("Import assignment");
    expect(markup).toContain("Assignments and project work stay on this computer");
    expect(markup).toContain("Enter Educator mode?");
    expect(markup).toContain("not a password or identity claim");
  });

  it("renders editable teacher comments with stable object and comment identities", () => {
    const comment = {
      body: "Check the holding branch on this rung.",
      commentId: "00000000-0000-4000-8000-000000001202",
      objectId: "00000000-0000-4000-8000-000000001201",
    } as const;
    const markup = renderToStaticMarkup(createElement(SubmissionReviewCommentEditor, {
      comments: [comment],
      disabled: false,
      onChange: () => undefined,
    }));

    expect(markup).toContain("Object and rung comments");
    expect(markup).toContain(comment.commentId.slice(0, 8));
    expect(markup).toContain(comment.objectId);
    expect(markup).toContain(comment.body);
    expect(markup).toContain("New focused comment");
  });

  it("renders structured teacher authoring controls without requiring raw JSON", () => {
    const markup = renderToStaticMarkup(createElement(TeacherAssignmentStudio, {
      disabled: false,
      draft: createBlankAssignmentDraft(sequentialIds()),
      onCaptureStarter: undefined,
      onChange: () => undefined,
      onClose: () => undefined,
      onExport: () => undefined,
      onPublish: () => undefined,
    }));

    expect(markup).toContain("Build a student mission");
    expect(markup).toContain("Title, summary &amp; objectives");
    expect(markup).toContain("Starter project &amp; hardware boundary");
    expect(markup).toContain("Starter tags");
    expect(markup).toContain("Permitted MVP instructions");
    expect(markup).toContain("Behavior tests");
    expect(markup).toContain("Progressive hints");
    expect(markup).toContain("Reset runtime");
    expect(markup).toContain("Set virtual value");
    expect(markup).toContain("Run PLC scans");
    expect(markup).toContain("Expect value");
    expect(markup).not.toContain("Paste assignment JSON");
  });
});

const sequentialIds = (): (() => string) => {
  let next = 1;
  return () => `00000000-0000-4000-8000-${String(next++).padStart(12, "0")}`;
};
