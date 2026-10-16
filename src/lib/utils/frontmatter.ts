/**
 * Turns parsed SKILL.md frontmatter (YAML values from gray-matter) into display nodes.
 * The YAML schema used by gray-matter already yields booleans, numbers and `Date` objects,
 * so this module only decides how each value should look.
 */

export type FrontmatterNode =
  | { kind: "text"; value: string }
  | { kind: "link"; href: string }
  | { kind: "tokens"; values: string[] }
  | { kind: "literal"; value: string }
  | { kind: "empty" }
  | { kind: "list"; items: FrontmatterNode[] }
  | { kind: "map"; entries: FrontmatterEntry[] };

export interface FrontmatterEntry {
  key: string;
  node: FrontmatterNode;
}

/**
 * Fields whose string value is a space- or comma-separated list
 * (Agent Skills `allowed-tools`, Claude Code `disallowed-tools`, `paths`, `arguments`).
 */
const TOKEN_LIST_FIELDS = new Set(["allowed-tools", "disallowed-tools", "paths", "arguments"]);

const URL_PATTERN = /^https?:\/\/\S+$/i;

function isScalar(value: unknown): value is string | number | boolean {
  return typeof value === "string" || typeof value === "number" || typeof value === "boolean";
}

function splitTokens(value: string): string[] {
  return value
    .split(/[\s,]+/)
    .map((token) => token.trim())
    .filter(Boolean);
}

function dateNode(value: Date): FrontmatterNode {
  if (Number.isNaN(value.getTime())) return { kind: "empty" };
  const iso = value.toISOString();
  // A bare `2026-06-25` parses to midnight UTC; show the date only in that case.
  return { kind: "literal", value: iso.endsWith("T00:00:00.000Z") ? iso.slice(0, 10) : iso };
}

export function toFrontmatterNode(key: string, value: unknown): FrontmatterNode {
  if (value === null || value === undefined) return { kind: "empty" };
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (!trimmed) return { kind: "empty" };
    if (TOKEN_LIST_FIELDS.has(key)) return { kind: "tokens", values: splitTokens(trimmed) };
    if (URL_PATTERN.test(trimmed)) return { kind: "link", href: trimmed };
    return { kind: "text", value: value.trimEnd() };
  }
  if (typeof value === "boolean" || typeof value === "number" || typeof value === "bigint") {
    return { kind: "literal", value: String(value) };
  }
  if (value instanceof Date) return dateNode(value);
  if (Array.isArray(value)) {
    if (value.length === 0) return { kind: "empty" };
    if (value.every(isScalar)) return { kind: "tokens", values: value.map(String) };
    return { kind: "list", items: value.map((item) => toFrontmatterNode(key, item)) };
  }
  if (typeof value === "object") {
    const entries = toFrontmatterEntries(value as Record<string, unknown>);
    return entries.length ? { kind: "map", entries } : { kind: "empty" };
  }
  return { kind: "text", value: String(value) };
}

/** Keeps the document order of the keys. */
export function toFrontmatterEntries(data: Record<string, unknown>): FrontmatterEntry[] {
  return Object.entries(data).map(([key, value]) => ({ key, node: toFrontmatterNode(key, value) }));
}
