const URL_SCHEME_PATTERN = /^[a-z][a-z\d+.-]*:/i;

/**
 * Resolves a link in the file `baseFile` to a path in the skill, without the section and query.
 * Returns `null` for a link with no path or a path outside the skill.
 */
export function resolveSkillPath(baseFile: string, target: string): string | null {
  const withoutHash = target.split("#")[0]?.split("?")[0] ?? "";
  if (!withoutHash) return null;
  if (withoutHash.startsWith("/")) {
    return withoutHash.replace(/^\/+/, "");
  }
  const resolved = baseFile.split("/").slice(0, -1);
  for (const segment of withoutHash.split("/")) {
    if (!segment || segment === ".") continue;
    if (segment === "..") {
      if (resolved.length === 0) return null;
      resolved.pop();
      continue;
    }
    resolved.push(segment);
  }
  return resolved.join("/");
}

/** The section of a link (the part after `#`), URL-decoded, or `""` when there is none. */
export function linkAnchor(href: string): string {
  const index = href.indexOf("#");
  if (index < 0) return "";
  const anchor = href.slice(index + 1);
  try {
    return decodeURIComponent(anchor);
  } catch {
    return anchor;
  }
}

/**
 * The Link preview text: where a click on `href` goes.
 * Without `baseFile`, a relative link shows as written.
 */
export function linkPreviewText(href: string, baseFile?: string): string {
  if (URL_SCHEME_PATTERN.test(href)) return href;
  // The renderer URL-encodes the section, e.g. a CJK heading slug.
  const section = href.includes("#") ? `#${linkAnchor(href)}` : "";
  if (href.startsWith("#")) return section;
  if (baseFile === undefined) return href;
  const path = resolveSkillPath(baseFile, href);
  return path === null ? href : `${path}${section}`;
}
