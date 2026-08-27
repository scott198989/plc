import { fileURLToPath } from "node:url";
import path from "node:path";

export const PROJECT_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../..",
);

export const resolveInProject = (...segments) => {
  const resolved = path.resolve(PROJECT_ROOT, ...segments);
  const relative = path.relative(PROJECT_ROOT, resolved);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error(`Resolved path leaves the project root: ${resolved}`);
  }
  return resolved;
};
