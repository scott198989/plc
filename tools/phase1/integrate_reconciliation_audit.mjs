import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const auditPath = resolve(root, "docs/governance/PHASE_1_ADVERSARIAL_AUDIT.md");
const reconciliationPath = resolve(root, "requirements/phase1-reconciliation.json");

function escapeCell(value) {
  return String(value ?? "")
    .replaceAll("\r\n", "\n")
    .replaceAll("\n", "<br>")
    .replaceAll("|", "\\|");
}

function fencedMarker(name, body) {
  return `<!-- ${name}_START -->\n\n${body.trim()}\n\n<!-- ${name}_END -->`;
}

function replaceMarked(document, name, body) {
  const start = `<!-- ${name}_START -->`;
  const end = `<!-- ${name}_END -->`;
  const startIndex = document.indexOf(start);
  const endIndex = document.indexOf(end);
  if (startIndex < 0 || endIndex < startIndex) {
    throw new Error(`Missing or invalid marker pair: ${name}`);
  }
  return (
    document.slice(0, startIndex) +
    fencedMarker(name, body) +
    document.slice(endIndex + end.length)
  );
}

function insertBeforeOnce(document, anchor, markerName, body) {
  if (document.includes(`<!-- ${markerName}_START -->`)) {
    return replaceMarked(document, markerName, body);
  }
  const index = document.indexOf(anchor);
  if (index < 0) throw new Error(`Insertion anchor not found: ${anchor}`);
  return (
    document.slice(0, index) +
    fencedMarker(markerName, body) +
    "\n\n" +
    document.slice(index)
  );
}

const reconciliation = JSON.parse(readFileSync(reconciliationPath, "utf8"));
const unitsById = new Map(reconciliation.sourceUnits.map((unit) => [unit.id, unit]));
const counts = reconciliation.counts;

const gapLines = [
  "### Final dispositions for all 48 reported recall gaps",
  "",
  "All 48 pre-remediation gaps are `MAPPED`; none was excluded. The source text remains verbatim in this table and the active IDs resolve in the schema-v3 registry.",
  "",
  "| Source unit | Page / section | Exact source text | Final disposition | Active atomic requirement ID(s) | Acceptance method |",
  "|---|---|---|---|---|---|",
];
for (const gap of reconciliation.gapDispositions) {
  const unit = unitsById.get(gap.sourceUnitId);
  if (!unit) throw new Error(`Gap references unknown source unit: ${gap.sourceUnitId}`);
  gapLines.push(
    `| \`${gap.sourceUnitId}\` | ${unit.page} / ${escapeCell(unit.section)} | ${escapeCell(unit.text)} | \`${gap.finalDisposition}\` | ${gap.requirementIds.map((id) => `\`${id}\``).join("<br>")} | ${escapeCell(gap.acceptanceMethods.join(" "))} |`,
  );
}

const ledgerLines = [];
const pages = new Map();
for (const unit of reconciliation.sourceUnits) {
  const page = pages.get(unit.page) ?? [];
  page.push(unit);
  pages.set(unit.page, page);
}
for (let pageNumber = 1; pageNumber <= 40; pageNumber += 1) {
  const pageUnits = pages.get(pageNumber) ?? [];
  ledgerLines.push(`#### Page ${pageNumber} — ${pageUnits.length} statement unit(s)`, "");
  if (pageUnits.length === 0) {
    ledgerLines.push("No in-scope statement unit was identified on this page.", "");
    continue;
  }
  ledgerLines.push(
    "| Unit | Section | Kind | Exact source text | Disposition | Active requirement ID(s) | Historical parent ID(s) |",
    "|---|---|---|---|---|---|---|",
  );
  for (const unit of pageUnits) {
    ledgerLines.push(
      `| \`${unit.id}\` | ${escapeCell(unit.section)} | \`${unit.kind}\` | ${escapeCell(unit.text)} | \`${unit.disposition}\` | ${unit.requirementIds.map((id) => `\`${id}\``).join("<br>")} | ${unit.historicalParentIds.map((id) => `\`${id}\``).join("<br>") || "—"} |`,
    );
  }
  ledgerLines.push("");
}

const splitLines = [
  "### 3.6 Post-remediation count and atomicity reconciliation",
  "",
  `- Source parent IDs preserved: **${counts.sourceParentIdCount}**.`,
  `- Total issued IDs after stable child allocation: **${counts.issuedIdCount}**.`,
  `- Historical superseded compound parents: **${counts.supersededCompoundParentCount}**.`,
  `- Atomic records: **${counts.atomicRecordCount}**; completion-eligible atomic records: **${counts.completionEligibleAtomicRecordCount}**.`,
  `- Independently walked source statement units: **${counts.sourceStatementUnitCount}**.`,
  `- Mapped / unmapped source units: **${counts.mappedStatementUnitCount} / ${counts.unmappedStatementUnitCount}**.`,
  `- Active source-unit-to-issued-ID relationships: **${counts.sourceUnitRelationshipCount}**.`,
  "",
  `Counting rule: ${reconciliation.method.mappingRule}`,
  "",
  "The earlier 770-edge estimate was an audit hypothesis, not an authority. Rebuilding the full graph exposed legitimate complete-register alias fan-out and produced 789 edges. Historical parent lineage is recorded separately and is not counted as an active mapping edge.",
  "",
  "#### Complete 20-parent / 190-child compound split ledger",
  "",
];
for (const split of reconciliation.compoundSplits) {
  splitLines.push(
    `##### ${split.parentId} → ${split.childIds.join(", ")}`,
    "",
    `Historical parent source text: ${escapeCell(split.sourceVerbatim)}`,
    "",
    "| Ordinal | Atomic child ID | Exact governed clause |",
    "|---:|---|---|",
  );
  for (const clause of split.clauses) {
    splitLines.push(
      `| ${clause.ordinal} | \`${clause.childId}\` | ${escapeCell(clause.verbatim)} |`,
    );
  }
  splitLines.push("");
}

let audit = readFileSync(auditPath, "utf8");
audit = insertBeforeOnce(
  audit,
  "### Complete source recall ledger",
  "FINAL_GAP_DISPOSITIONS",
  gapLines.join("\n"),
);
audit = replaceMarked(audit, "LEDGER", ledgerLines.join("\n"));
audit = insertBeforeOnce(
  audit,
  "## Task 4 — Baseline and hash integrity",
  "COMPOUND_SPLIT_LEDGER",
  splitLines.join("\n"),
);

writeFileSync(auditPath, audit.replaceAll("\r\n", "\n"), "utf8");
console.log(
  `Integrated ${reconciliation.gapDispositions.length} final gap dispositions, ${reconciliation.sourceUnits.length} source units, and ${reconciliation.compoundSplits.length} compound splits into ${auditPath}`,
);
