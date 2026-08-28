const MAXIMUM_DEPTH = 64;

/** Encodes bounded data with ordinal key order and no locale-dependent values. */
export const encodeCanonicalJson = (value: unknown): Uint8Array<ArrayBuffer> =>
  new TextEncoder().encode(stringifyCanonical(value, 0));

const stringifyCanonical = (value: unknown, depth: number): string => {
  if (depth > MAXIMUM_DEPTH) {
    throw new Error("Canonical JSON exceeds the nesting limit.");
  }
  if (value === null) {
    return "null";
  }
  switch (typeof value) {
    case "boolean":
      return value ? "true" : "false";
    case "string":
      return JSON.stringify(value);
    case "number":
      if (!Number.isSafeInteger(value)) {
        throw new Error("Canonical JSON numbers must be safe integers.");
      }
      return String(value);
    case "object":
      if (Array.isArray(value)) {
        return `[${value.map((item) => stringifyCanonical(item, depth + 1)).join(",")}]`;
      }
      if (Object.getPrototypeOf(value) !== Object.prototype) {
        throw new Error("Canonical JSON accepts only plain records and arrays.");
      }
      return `{${Object.entries(value as Readonly<Record<string, unknown>>)
        .sort(([left], [right]) => ordinalCompare(left, right))
        .map(([key, item]) => {
          if (item === undefined) {
            throw new Error("Canonical JSON does not accept undefined fields.");
          }
          return `${JSON.stringify(key)}:${stringifyCanonical(item, depth + 1)}`;
        })
        .join(",")}}`;
    default:
      throw new Error(`Canonical JSON does not accept ${typeof value} values.`);
  }
};

const ordinalCompare = (left: string, right: string): number => {
  if (left === right) {
    return 0;
  }
  return left < right ? -1 : 1;
};
