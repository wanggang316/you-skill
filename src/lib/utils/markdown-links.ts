const URL_SCHEME_PATTERN = /^[a-z][a-z\d+.-]*:/i;

/** Decodes a URL-encoded part of a link; keeps it as written when the encoding is invalid. */
function decodeLinkPart(part: string): string {
  try {
    return decodeURIComponent(part);
  } catch {
    return part;
  }
}

/**
 * Resolves a link in the file `baseFile` to a path in the skill, without the section and query.
 * The renderer URL-encodes the link, so each path segment is decoded to match the file names.
 * Returns `null` for a link with no path or a path outside the skill.
 */
export function resolveSkillPath(baseFile: string, target: string): string | null {
  const withoutHash = target.split("#")[0]?.split("?")[0] ?? "";
  if (!withoutHash) return null;
  const segments = withoutHash.split("/").map(decodeLinkPart);
  if (withoutHash.startsWith("/")) {
    return segments.join("/").replace(/^\/+/, "");
  }
  const resolved = baseFile.split("/").slice(0, -1);
  for (const segment of segments) {
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
  return decodeLinkPart(href.slice(index + 1));
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
