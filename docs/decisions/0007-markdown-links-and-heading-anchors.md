# Markdown links and heading anchors

Status: Accepted

## Context

`MarkdownPreview` renders Markdown in three places: the skill file viewer, the preview of the Markdown editor, and the update dialog. Two problems exist:

- The user cannot see where a link goes before a click. A browser shows the target in a small status text at the bottom left of the window on hover.
- A click on a link to a section of the same file (`#section`) does nothing. The renderer (`marked`) gives headings no `id`, and `MarkdownPreview` lets the webview handle `#` links. A link to a section of another file (`other.md#section`) opens the file but drops the section.

Most skill documents are written on GitHub, so their authors use the heading anchors that GitHub makes.

## Decision

- **Link preview**: when the pointer is on a link in `MarkdownPreview` (the body and the frontmatter table), show the link target in a small text at the bottom left of the app window (`position: fixed`). It does not scroll with the content. Hide it when the pointer leaves the link. This applies in all three places, also in the update dialog, where the text shows outside the dialog, as in a browser.
- **Preview text**: show where a click goes.
  - External link: the full URL.
  - Link to a section of the same file: `#<slug>`.
  - Relative file link: the path in the skill after resolution, with the section if there is one, for example `references/api.md#auth`.
- **Heading ids**: give each heading an `id` with the GitHub slug rules (`marked-gfm-heading-id`, which uses `github-slugger`). A duplicate heading gets a `-1`, `-2` suffix. CJK text stays in the slug, as on GitHub.
- **Translation views**:
  - Bilingual view: only the original heading gets an `id`. The translated line of a heading gets no `id`.
  - Translation-only view: if the translated file has the same number of headings as the original, heading N gets the `id` of original heading N. Otherwise make the ids from the translated text.
- **Jump**: a click on `#<slug>` scrolls the scroll container of the preview smoothly to the target, with a small top margin. It does not change the URL hash or the history. If no target exists, do nothing and log a warning to the console.
- **Targets**: find the target by `id` first, then by the `name` of an `<a>` element, so raw HTML anchors (`<a id="x">`, `<a name="x">`) also work.
- **Link to another file with a section** (`other.md#section`): load the file, and after it renders, scroll to the section with the same jump rules.

## Consequences

- The app gets one new small dependency (`marked-gfm-heading-id`).
- A jump has no "back" action.
- In the translation-only view, a link whose slug the translation changed can have no target when the heading counts are different.

## Reconsider when

- Users want to go back after a jump. Then keep a jump history in the preview.
- Documents from other hosts (for example GitLab) use different slug rules and their links fail.
