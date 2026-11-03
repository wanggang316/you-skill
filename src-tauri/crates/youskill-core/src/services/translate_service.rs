use crate::config::{config_path, load_config, AppConfig};
use crate::services::ai_service;
use crate::utils::file::split_frontmatter;
use serde_yaml::{Mapping, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};

fn build_translation_system_prompt(target_language: &str) -> String {
  format!(
    concat!(
      "You are a professional technical translator for Markdown documentation.\n",
      "Translate the user-provided markdown into {target_language}.\n",
      "Rules:\n",
      "1. Keep markdown structure exactly (headings, lists, tables, blockquotes).\n",
      "2. Keep code fences, inline code, file paths, commands, URLs, env var names unchanged.\n",
      "3. Keep YAML frontmatter keys unchanged; translate only human-readable values.\n",
      "4. Do not add explanations, notes, or surrounding markdown/code fences.\n",
      "5. Return only the translated markdown content."
    ),
    target_language = target_language
  )
}

fn build_translation_cache_key(model: &str, target_language: &str, markdown: &str) -> String {
  let mut hasher = Sha256::new();
  hasher.update(markdown.as_bytes());
  let markdown_hash = format!("{:x}", hasher.finalize());
  let raw = format!(
    "v1|model={}|lang={}|hash={}",
    model.trim(),
    target_language.trim(),
    markdown_hash
  );
  let mut key_hasher = Sha256::new();
  key_hasher.update(raw.as_bytes());
  format!("{:x}", key_hasher.finalize())
}

fn translation_cache_dir_path() -> Result<PathBuf, String> {
  let base = config_path()?;
  let parent = base
    .parent()
    .ok_or_else(|| "无法获取翻译缓存目录".to_string())?;
  Ok(parent.join("translate_cache"))
}

fn translation_cache_file_path(cache_dir: &Path, cache_key: &str) -> PathBuf {
  cache_dir.join(format!("{}.md", cache_key))
}

fn load_translation_cache_entry(cache_dir: &Path, cache_key: &str) -> Option<String> {
  let path = translation_cache_file_path(cache_dir, cache_key);
  if !path.exists() {
    return None;
  }

  let content = match fs::read_to_string(&path) {
    Ok(content) => content,
    Err(error) => {
      tracing::warn!(
        "Failed to read translation cache file {}: {}",
        path.display(),
        error
      );
      return None;
    },
  };

  if content.trim().is_empty() {
    return None;
  }
  Some(content)
}

fn build_description_system_prompt(target_language: &str) -> String {
  format!(
    concat!(
      "You are a professional technical translator.\n",
      "Translate the user-provided text into {target_language}.\n",
      "Keep inline code, file paths, commands, URLs, and env var names unchanged.\n",
      "Return only the translated text, without quotes, explanations, or code fences."
    ),
    target_language = target_language
  )
}

/// A model can echo the source text back. Such a result is not a translation.
fn is_untranslated(markdown: &str, translated: &str) -> bool {
  markdown.trim() == translated.trim()
}

const THINK_END_TAG: &str = "</think>";
const THINKING_PREAMBLE: &str = "Thinking Process:";

/// Removes the reasoning and code fence that some models put around the translation.
/// `source` is the text sent to the model; a form that the source itself has is kept.
/// An output without a wrapper is returned unchanged.
fn strip_model_wrapper(output: &str, source: &str) -> String {
  let mut text = output;
  let mut changed = false;

  if !source.contains(THINK_END_TAG) {
    if let Some(end) = text.rfind(THINK_END_TAG) {
      text = &text[end + THINK_END_TAG.len()..];
      changed = true;
    }
  }

  let trimmed = text.trim_start();
  if trimmed.starts_with(THINKING_PREAMBLE) && !source.trim_start().starts_with(THINKING_PREAMBLE) {
    if let Some(start) = document_start(trimmed, source) {
      text = &trimmed[start..];
      changed = true;
    }
  }

  if let Some(inner) = fenced_document(text, source) {
    text = inner;
    changed = true;
  }

  if changed {
    text.trim_start_matches(['\r', '\n']).to_string()
  } else {
    output.to_string()
  }
}

/// Byte offset of the line in `text` that starts the document, found from the first line
/// of `source`: the frontmatter opening line, or a heading of the same level. The first
/// line of `text` is the preamble and is never the start.
fn document_start(text: &str, source: &str) -> Option<usize> {
  let first = source
    .lines()
    .map(str::trim_end)
    .find(|line| !line.is_empty())?;
  let matches: Box<dyn Fn(&str) -> bool> = if first == "---" {
    Box::new(|line: &str| line.trim_end() == "---")
  } else if first.starts_with('#') {
    let level = first.chars().take_while(|c| *c == '#').count();
    let prefix = format!("{} ", "#".repeat(level));
    Box::new(move |line: &str| line.starts_with(&prefix))
  } else {
    return None;
  };

  let mut offset = 0;
  for (index, line) in text.split_inclusive('\n').enumerate() {
    if index > 0 && matches(line) {
      return Some(offset);
    }
    offset += line.len();
  }
  None
}

/// The text inside a ```` ```markdown ````, ```` ```md ```` or ```` ``` ```` fence that wraps
/// all of `text`. `None` when `text` is not wrapped or `source` itself starts with a fence.
/// The translation keeps code blocks, so a wrapped `text` has exactly two fence lines more
/// than `source`; this separates a wrapper from a document that starts and ends with code.
fn fenced_document<'a>(text: &'a str, source: &str) -> Option<&'a str> {
  let fence_lines = |value: &str| {
    value
      .lines()
      .filter(|line| line.trim_start().starts_with("```"))
      .count()
  };
  if source.trim_start().starts_with("```") || fence_lines(text) != fence_lines(source) + 2 {
    return None;
  }
  let trimmed = text.trim();
  let (first, rest) = trimmed.split_once('\n')?;
  let info = first.strip_prefix("```")?.trim().to_ascii_lowercase();
  if !matches!(info.as_str(), "" | "markdown" | "md") {
    return None;
  }
  let inner = rest.trim_end().strip_suffix("```")?;
  if !inner.is_empty() && !inner.ends_with('\n') {
    return None;
  }
  Some(inner)
}

/// A Markdown file split around its `---` frontmatter, with the delimiter lines kept as written.
struct FrontmatterDocument<'a> {
  opening: &'a str,
  frontmatter: &'a str,
  closing: &'a str,
  body: &'a str,
}

fn split_document(markdown: &str) -> Option<FrontmatterDocument<'_>> {
  let (frontmatter, body) = split_frontmatter(markdown)?;
  let opening_len = markdown.split_inclusive('\n').next()?.len();
  let closing_end = markdown.len() - body.len();
  Some(FrontmatterDocument {
    opening: &markdown[..opening_len],
    frontmatter,
    closing: &markdown[opening_len + frontmatter.len()..closing_end],
    body,
  })
}

/// The `description` value of the frontmatter when it is a non-blank string.
fn frontmatter_description(frontmatter: &str) -> Option<String> {
  let yaml: Value = serde_yaml::from_str(frontmatter).ok()?;
  yaml
    .get("description")
    .and_then(Value::as_str)
    .filter(|value| !value.trim().is_empty())
    .map(str::to_string)
}

/// Returns `frontmatter` with the `description` value set to `description`. Only the lines of
/// the top-level `description` entry change; other lines stay as written. When those lines
/// cannot be found, the whole mapping is written again with the same keys and values.
fn replace_frontmatter_description(frontmatter: &str, description: &str) -> Option<String> {
  let mut mapping: Mapping = serde_yaml::from_str(frontmatter).ok()?;
  let key = Value::String("description".to_string());
  if !mapping.get(&key)?.is_string() {
    return None;
  }
  mapping.insert(key, Value::String(description.to_string()));
  let expected = Value::Mapping(mapping);

  let edited = replace_description_lines(frontmatter, description)
    .filter(|edited| serde_yaml::from_str::<Value>(edited).ok().as_ref() == Some(&expected));
  match edited {
    Some(edited) => Some(edited),
    None => serde_yaml::to_string(&expected).ok(),
  }
}

/// Replaces the lines of the top-level `description` entry, including the indented lines of a
/// block or multi-line scalar, with one double-quoted value.
fn replace_description_lines(frontmatter: &str, description: &str) -> Option<String> {
  let lines: Vec<&str> = frontmatter.split_inclusive('\n').collect();
  let start = lines.iter().position(|line| {
    line
      .strip_prefix("description")
      .is_some_and(|rest| rest.trim_start_matches([' ', '\t']).starts_with(':'))
  })?;
  let mut end = start + 1;
  let mut index = start + 1;
  while index < lines.len() {
    let line = lines[index];
    if line.trim().is_empty() {
      index += 1;
    } else if line.starts_with([' ', '\t']) {
      index += 1;
      end = index;
    } else {
      break;
    }
  }

  let line_ending = if lines[start].ends_with("\r\n") {
    "\r\n"
  } else {
    "\n"
  };
  // A JSON string is a valid YAML double-quoted scalar.
  let quoted = serde_json::to_string(description).ok()?;
  Some(format!(
    "{}description: {}{}{}",
    lines[..start].concat(),
    quoted,
    line_ending,
    lines[end..].concat()
  ))
}

/// Builds the translated file: the original frontmatter with the translated description and
/// the translated body, with the original whitespace around the body.
fn assemble_translation(
  document: &FrontmatterDocument<'_>,
  translated_description: Option<&str>,
  translated_body: &str,
) -> String {
  let frontmatter = translated_description
    .and_then(|description| replace_frontmatter_description(document.frontmatter, description))
    .unwrap_or_else(|| document.frontmatter.to_string());
  let leading_len = document.body.len() - document.body.trim_start().len();
  let trailing = if document.body.ends_with('\n') && !translated_body.trim().is_empty() {
    "\n"
  } else {
    ""
  };
  format!(
    "{}{}{}{}{}{}",
    document.opening,
    frontmatter,
    document.closing,
    &document.body[..leading_len],
    translated_body.trim(),
    trailing
  )
}

fn save_translation_cache_entry(
  cache_dir: &Path,
  cache_key: &str,
  translated_markdown: &str,
) -> Result<(), String> {
  fs::create_dir_all(cache_dir).map_err(|error| format!("创建翻译缓存目录失败: {}", error))?;
  let path = translation_cache_file_path(cache_dir, cache_key);
  fs::write(path, translated_markdown).map_err(|error| format!("写入翻译缓存失败: {}", error))
}

/// Returns the cached translation for `markdown` under the current model and target
/// language. Never calls the translation API.
pub fn cached_skill_translation(markdown: &str) -> Result<Option<String>, String> {
  let config = load_config()?;
  Ok(lookup_cached_translation(
    &config,
    &translation_cache_dir_path()?,
    markdown,
  ))
}

fn lookup_cached_translation(
  config: &AppConfig,
  cache_dir: &Path,
  markdown: &str,
) -> Option<String> {
  let target_language = config.translate_target_language.trim();
  let model = config.translate_model.trim();
  if markdown.trim().is_empty() || target_language.is_empty() || model.is_empty() {
    return None;
  }
  let cache_key = build_translation_cache_key(model, target_language, markdown);
  // Entries saved by earlier versions can keep a model wrapper.
  load_translation_cache_entry(cache_dir, &cache_key)
    .and_then(|cached| checked_translation(&cached, markdown).ok())
}

/// Why a model result cannot be used as the translation of its source.
#[derive(Debug, PartialEq)]
enum UnusableTranslation {
  Blank,
  Untranslated,
  /// A reasoning preamble that `strip_model_wrapper` could not separate from the text.
  Reasoning,
}

#[derive(Debug, PartialEq)]
enum TranslationFailure {
  Request(String),
  Unusable(UnusableTranslation),
}

/// Removes model wrappers from `output` and checks that the rest is a translation of `source`.
fn checked_translation(output: &str, source: &str) -> Result<String, UnusableTranslation> {
  let cleaned = strip_model_wrapper(output, source);
  if cleaned.trim().is_empty() {
    return Err(UnusableTranslation::Blank);
  }
  if cleaned.trim_start().starts_with(THINKING_PREAMBLE)
    && !source.trim_start().starts_with(THINKING_PREAMBLE)
  {
    return Err(UnusableTranslation::Reasoning);
  }
  if is_untranslated(source, &cleaned) {
    return Err(UnusableTranslation::Untranslated);
  }
  Ok(cleaned)
}

/// Calls `request` up to `attempts` times and returns the first usable translation of `source`.
async fn translate_with_retry<F, Fut>(
  source: &str,
  attempts: usize,
  mut request: F,
) -> Result<String, TranslationFailure>
where
  F: FnMut() -> Fut,
  Fut: Future<Output = Result<String, String>>,
{
  let mut failure = TranslationFailure::Unusable(UnusableTranslation::Blank);
  for _ in 0..attempts {
    let output = request().await.map_err(TranslationFailure::Request)?;
    match checked_translation(&output, source) {
      Ok(translated) => return Ok(translated),
      Err(reason) => failure = TranslationFailure::Unusable(reason),
    }
  }
  Err(failure)
}

async fn translate_text(
  api_key: String,
  model: String,
  system_prompt: String,
  text: String,
  attempts: usize,
) -> Result<String, TranslationFailure> {
  translate_with_retry(&text, attempts, || {
    let (api_key, model) = (api_key.clone(), model.clone());
    let messages = vec![
      ai_service::OpenRouterMessage {
        role: "system".to_string(),
        content: system_prompt.clone(),
      },
      ai_service::OpenRouterMessage {
        role: "user".to_string(),
        content: text.clone(),
      },
    ];
    async move {
      ai_service::chat_completion_with_openrouter(&api_key, &model, messages, 0.1).await
    }
  })
  .await
}

fn translation_failure_message(
  failure: TranslationFailure,
  part: &str,
  model: &str,
  target_language: &str,
) -> String {
  match failure {
    TranslationFailure::Request(error) => error,
    TranslationFailure::Unusable(UnusableTranslation::Blank) => {
      format!("模型 {} 返回的{}为空。请重试或更换模型", model, part)
    },
    TranslationFailure::Unusable(UnusableTranslation::Untranslated) => format!(
      "模型 {} 返回的{}与原文相同，未翻译为 {}。请检查目标语言或更换模型后重试",
      model, part, target_language
    ),
    TranslationFailure::Unusable(UnusableTranslation::Reasoning) => format!(
      "模型 {} 返回了推理过程，而不是{}的译文。请重试或更换模型",
      model, part
    ),
  }
}

/// A short description costs little to request again; the body can take a long time.
const BODY_ATTEMPTS: usize = 1;
const DESCRIPTION_ATTEMPTS: usize = 2;

/// Translates `markdown` with the configured model. When `force` is true, skips the
/// cached translation and replaces it with the new result.
pub async fn translate_skill_markdown(markdown: String, force: bool) -> Result<String, String> {
  if markdown.trim().is_empty() {
    return Ok(markdown);
  }

  let config = load_config()?;
  let target_language = config.translate_target_language.trim().to_string();
  if target_language.is_empty() {
    return Err("Target language 未配置，请先在设置中填写".to_string());
  }
  let model = config.translate_model.trim().to_string();
  if model.is_empty() {
    return Err("Model 未配置，请先在设置中填写".to_string());
  }
  let cache_dir = translation_cache_dir_path()?;
  if !force {
    if let Some(cached_markdown) = lookup_cached_translation(&config, &cache_dir, &markdown) {
      return Ok(cached_markdown);
    }
  }

  let api_key = config
    .openrouter_api_key
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .ok_or_else(|| "OpenRouter API Key 未配置，请先在设置中填写".to_string())?
    .to_string();

  // The model does not reliably translate frontmatter values, so the body and the
  // description are translated separately and the frontmatter is kept as written.
  let document = split_document(&markdown);
  let description = document
    .as_ref()
    .and_then(|document| frontmatter_description(document.frontmatter));
  let body = document
    .as_ref()
    .map_or(markdown.as_str(), |document| document.body.trim());

  let description_task = description.map(|description| {
    tokio::spawn(translate_text(
      api_key.clone(),
      model.clone(),
      build_description_system_prompt(&target_language),
      description,
      DESCRIPTION_ATTEMPTS,
    ))
  });
  let translated_body = if body.trim().is_empty() {
    body.to_string()
  } else {
    translate_text(
      api_key.clone(),
      model.clone(),
      build_translation_system_prompt(&target_language),
      body.to_string(),
      BODY_ATTEMPTS,
    )
    .await
    .map_err(|failure| translation_failure_message(failure, "内容", &model, &target_language))?
  };
  // A translation is saved only with a usable description translation.
  let translated_description = match description_task {
    Some(task) => Some(
      task
        .await
        .map_err(|error| format!("翻译 description 失败: {}", error))?
        .map_err(|failure| {
          translation_failure_message(failure, " description ", &model, &target_language)
        })?,
    ),
    None => None,
  };

  let content = match &document {
    Some(document) => assemble_translation(
      document,
      translated_description.as_deref().map(str::trim),
      &translated_body,
    ),
    None => translated_body,
  };

  let cache_key = build_translation_cache_key(&model, &target_language, &markdown);
  if let Err(error) = save_translation_cache_entry(&cache_dir, &cache_key, &content) {
    tracing::warn!("Failed to save translation cache: {}", error);
  }

  Ok(content)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn translate_config() -> AppConfig {
    AppConfig {
      translate_target_language: "Chinese".to_string(),
      translate_model: "test/model".to_string(),
      ..AppConfig::default()
    }
  }

  #[test]
  fn lookup_returns_saved_translation() {
    let dir = tempfile::tempdir().unwrap();
    let config = translate_config();
    let key = build_translation_cache_key("test/model", "Chinese", "# Title");
    save_translation_cache_entry(dir.path(), &key, "# 标题").unwrap();

    assert_eq!(
      lookup_cached_translation(&config, dir.path(), "# Title").as_deref(),
      Some("# 标题")
    );
  }

  #[test]
  fn lookup_misses_when_markdown_or_settings_change() {
    let dir = tempfile::tempdir().unwrap();
    let config = translate_config();
    let key = build_translation_cache_key("test/model", "Chinese", "# Title");
    save_translation_cache_entry(dir.path(), &key, "# 标题").unwrap();

    assert_eq!(
      lookup_cached_translation(&config, dir.path(), "# Other"),
      None
    );
    let other_model = AppConfig {
      translate_model: "other/model".to_string(),
      ..translate_config()
    };
    assert_eq!(
      lookup_cached_translation(&other_model, dir.path(), "# Title"),
      None
    );
    let unconfigured = AppConfig::default();
    assert_eq!(
      lookup_cached_translation(&unconfigured, dir.path(), "# Title"),
      None
    );
  }

  #[test]
  fn lookup_ignores_untranslated_cache_entry() {
    let dir = tempfile::tempdir().unwrap();
    let config = translate_config();
    let key = build_translation_cache_key("test/model", "Chinese", "# Title\n");
    save_translation_cache_entry(dir.path(), &key, "# Title").unwrap();

    assert_eq!(
      lookup_cached_translation(&config, dir.path(), "# Title\n"),
      None
    );
  }

  #[test]
  fn untranslated_ignores_surrounding_whitespace_only() {
    assert!(is_untranslated("# Title\n", "  # Title"));
    assert!(!is_untranslated("# Title", "# 标题"));
  }

  #[test]
  fn lookup_ignores_blank_cache_entry() {
    let dir = tempfile::tempdir().unwrap();
    let config = translate_config();
    let key = build_translation_cache_key("test/model", "Chinese", "# Title");
    save_translation_cache_entry(dir.path(), &key, "  \n").unwrap();

    assert_eq!(
      lookup_cached_translation(&config, dir.path(), "# Title"),
      None
    );
  }

  const SKILL_MD: &str = "---\nname: demo\ndescription: Use when a user asks for a demo.\nlicense: MIT\n---\n\n# Demo\n\nRun the demo.\n";

  fn parsed_frontmatter(markdown: &str) -> Value {
    let (frontmatter, _) = split_frontmatter(markdown).expect("frontmatter");
    serde_yaml::from_str(frontmatter).expect("valid YAML")
  }

  fn translated(markdown: &str, description: &str, body: &str) -> String {
    let document = split_document(markdown).expect("frontmatter");
    assemble_translation(&document, Some(description), body)
  }

  #[test]
  fn assemble_puts_translated_description_into_original_frontmatter() {
    let result = translated(SKILL_MD, "当用户要求演示时使用。", "# 演示\n\n运行演示。");

    assert_eq!(
      result,
      "---\nname: demo\ndescription: \"当用户要求演示时使用。\"\nlicense: MIT\n---\n\n# 演示\n\n运行演示。\n"
    );
  }

  #[test]
  fn assemble_keeps_other_fields_and_key_order() {
    let markdown = "---\nname: demo\ndescription: >\n  Folded text\n  over lines.\nmetadata:\n  author: someone\n  tags: [a, b]\nallowed-tools: Read, Grep\n---\nBody\n";
    let result = translated(markdown, "折叠文本。", "正文");

    let original = parsed_frontmatter(markdown);
    let yaml = parsed_frontmatter(&result);
    let original_keys: Vec<_> = original.as_mapping().unwrap().keys().collect();
    let keys: Vec<_> = yaml.as_mapping().unwrap().keys().collect();
    assert_eq!(keys, original_keys);
    for key in ["name", "metadata", "allowed-tools"] {
      assert_eq!(yaml.get(key), original.get(key), "field {}", key);
    }
    assert_eq!(yaml["description"].as_str(), Some("折叠文本。"));
    assert!(
      result.contains("metadata:\n  author: someone\n  tags: [a, b]\nallowed-tools: Read, Grep\n")
    );
    assert!(result.ends_with("---\n正文\n"));
  }

  #[test]
  fn description_edge_cases_round_trip_into_valid_yaml() {
    let cases = [
      (
        "description: |\n  Line one.\n  Line two.\n",
        "第一行。\n第二行。",
      ),
      ("description: >-\n  Folded\n  text.\n", "折叠：文本"),
      ("description: \"Quoted: value\"\n", "带引号的 \"值\""),
      ("description: 'Single ''quoted'''\n", "单引号 'quoted'"),
      (
        "description: Plain text\n  continued here.\n",
        "key: value # not a comment",
      ),
      ("description:   Spaced\n", "- 列表样式\t制表符\\反斜杠"),
    ];
    for (entry, translation) in cases {
      let markdown = format!("---\nname: demo\n{}version: 1\n---\nBody\n", entry);
      let result = translated(&markdown, translation, "正文");
      let yaml = parsed_frontmatter(&result);
      assert_eq!(
        yaml["description"].as_str(),
        Some(translation),
        "entry {:?}",
        entry
      );
      assert_eq!(yaml["name"].as_str(), Some("demo"));
      assert_eq!(yaml["version"].as_u64(), Some(1));
      assert!(result.contains("\nversion: 1\n"), "entry {:?}", entry);
    }
  }

  #[test]
  fn assemble_keeps_crlf_line_endings() {
    let markdown = "---\r\nname: demo\r\ndescription: Text\r\n---\r\nBody\r\n";
    let result = translated(markdown, "文本", "正文");
    assert_eq!(
      result,
      "---\r\nname: demo\r\ndescription: \"文本\"\r\n---\r\n正文\n"
    );
  }

  #[test]
  fn replace_falls_back_to_rewritten_mapping_when_lines_are_not_found() {
    let frontmatter = "{name: demo, description: Text}\n";
    let result = replace_frontmatter_description(frontmatter, "文本").unwrap();
    let yaml: Value = serde_yaml::from_str(&result).unwrap();
    assert_eq!(yaml["name"].as_str(), Some("demo"));
    assert_eq!(yaml["description"].as_str(), Some("文本"));
  }

  #[test]
  fn replace_ignores_frontmatter_without_string_description() {
    assert_eq!(
      replace_frontmatter_description("name: demo\n", "文本"),
      None
    );
    assert_eq!(
      replace_frontmatter_description("description:\n  nested: true\n", "文本"),
      None
    );
    assert_eq!(
      frontmatter_description("name: demo\ndescription: \"  \"\n"),
      None
    );
    assert_eq!(
      frontmatter_description("description: Text\n").as_deref(),
      Some("Text")
    );
  }

  #[test]
  fn assemble_without_description_keeps_frontmatter() {
    let markdown = "---\nname: demo\n---\n# Title\n";
    let document = split_document(markdown).unwrap();
    assert_eq!(
      assemble_translation(&document, None, "# 标题"),
      "---\nname: demo\n---\n# 标题\n"
    );
  }

  #[test]
  fn strip_removes_think_block() {
    let output = "<think>\nThe user wants Chinese.\n</think>\n\n# 演示\n\n运行。";
    assert_eq!(
      strip_model_wrapper(output, "# Demo\n\nRun."),
      "# 演示\n\n运行。"
    );
  }

  #[test]
  fn strip_removes_reasoning_before_lone_think_end_tag() {
    let output = "</think>\n\n---\nname: demo\n---\n# 演示";
    assert_eq!(
      strip_model_wrapper(output, SKILL_MD),
      "---\nname: demo\n---\n# 演示"
    );
    let output = "Okay, translate it.\n</think>\n# 演示";
    assert_eq!(strip_model_wrapper(output, "# Demo"), "# 演示");
  }

  #[test]
  fn strip_removes_thinking_process_preamble() {
    let output = "Thinking Process:\n\n1. **Analyze:** a skill file.\n2. Keep `---`.\n\n---\nname: demo\ndescription: 演示\n---\n\n# 演示\n";
    assert_eq!(
      strip_model_wrapper(output, SKILL_MD),
      "---\nname: demo\ndescription: 演示\n---\n\n# 演示\n"
    );
    let output = "Thinking Process:\n\n1. Translate the heading.\n\n## 用法\n\n运行。";
    assert_eq!(
      strip_model_wrapper(output, "## Usage\n\nRun."),
      "## 用法\n\n运行。"
    );
  }

  #[test]
  fn strip_keeps_thinking_preamble_without_document_anchor() {
    let output = "Thinking Process:\n\n运行。";
    assert_eq!(strip_model_wrapper(output, "Run."), output);
  }

  #[test]
  fn strip_removes_markdown_fence() {
    for fence in ["```markdown", "```md", "```", "```Markdown"] {
      let output = format!("{}\n# 演示\n\n```bash\nrun\n```\n```\n", fence);
      assert_eq!(
        strip_model_wrapper(&output, "# Demo\n\n```bash\nrun\n```\n"),
        "# 演示\n\n```bash\nrun\n```\n",
        "fence {}",
        fence
      );
    }
  }

  #[test]
  fn strip_removes_think_block_then_fence() {
    let output = "<think>plan</think>\n```markdown\n# 演示\n```";
    assert_eq!(strip_model_wrapper(output, "# Demo"), "# 演示\n");
  }

  #[test]
  fn strip_keeps_output_without_wrapper() {
    for output in [
      "# 演示\n\n运行。\n",
      "\n---\nname: demo\n---\n# 演示",
      "```bash\nrun\n```\n\n正文",
      "```python\nprint()\n```",
      "```\nrun\n```\n\n正文\n\n```\nmore\n```",
    ] {
      assert_eq!(strip_model_wrapper(output, "# Demo"), output);
    }
  }

  #[test]
  fn strip_keeps_wrapper_forms_present_in_source() {
    let source = "```markdown\n# Demo\n```";
    let output = "```markdown\n# 演示\n```";
    assert_eq!(strip_model_wrapper(output, source), output);
    let source = "Explain the `</think>` tag.";
    let output = "解释 `</think>` 标签。";
    assert_eq!(strip_model_wrapper(output, source), output);
  }

  #[test]
  fn lookup_removes_wrapper_from_cached_entry() {
    let dir = tempfile::tempdir().unwrap();
    let config = translate_config();
    let key = build_translation_cache_key("test/model", "Chinese", SKILL_MD);
    let wrapped = "```markdown\n---\nname: demo\ndescription: 演示\n---\n# 演示\n```\n";
    save_translation_cache_entry(dir.path(), &key, wrapped).unwrap();

    assert_eq!(
      lookup_cached_translation(&config, dir.path(), SKILL_MD).as_deref(),
      Some("---\nname: demo\ndescription: 演示\n---\n# 演示\n")
    );
  }

  fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
      .build()
      .unwrap()
      .block_on(future)
  }

  /// Runs `translate_with_retry` with fake model outputs and returns the result and call count.
  fn retry_with_outputs(
    source: &str,
    attempts: usize,
    outputs: &[Result<&str, &str>],
  ) -> (Result<String, TranslationFailure>, usize) {
    let mut calls = 0;
    let result = block_on(translate_with_retry(source, attempts, || {
      let output = outputs[calls].map(str::to_string).map_err(str::to_string);
      calls += 1;
      async move { output }
    }));
    (result, calls)
  }

  #[test]
  fn checked_translation_rejects_unusable_results() {
    assert_eq!(
      checked_translation(" \n", "Text"),
      Err(UnusableTranslation::Blank)
    );
    assert_eq!(
      checked_translation("Text\n", "Text"),
      Err(UnusableTranslation::Untranslated)
    );
    assert_eq!(
      checked_translation("<think>plan</think>\nText", "Text"),
      Err(UnusableTranslation::Untranslated)
    );
    assert_eq!(
      checked_translation("Thinking Process:\n\n1. Translate.\n\n文本", "Text"),
      Err(UnusableTranslation::Reasoning)
    );
    assert_eq!(
      checked_translation("```\n文本\n```", "Text").as_deref(),
      Ok("文本\n")
    );
  }

  #[test]
  fn retry_requests_description_again_after_echo() {
    let (result, calls) = retry_with_outputs(
      "Use for demos.",
      DESCRIPTION_ATTEMPTS,
      &[Ok("Use for demos."), Ok("用于演示。")],
    );
    assert_eq!(result.as_deref(), Ok("用于演示。"));
    assert_eq!(calls, 2);
  }

  #[test]
  fn retry_rejects_description_that_stays_unusable() {
    let (result, calls) = retry_with_outputs(
      "Use for demos.",
      DESCRIPTION_ATTEMPTS,
      &[Ok(""), Ok("Use for demos.")],
    );
    assert_eq!(
      result,
      Err(TranslationFailure::Unusable(
        UnusableTranslation::Untranslated
      ))
    );
    assert_eq!(calls, 2);

    let (result, _) = retry_with_outputs(
      "Use for demos.",
      DESCRIPTION_ATTEMPTS,
      &[
        Ok("Thinking Process:\n\n用于演示。"),
        Ok("Thinking Process:\n\n用于演示。"),
      ],
    );
    assert_eq!(
      result,
      Err(TranslationFailure::Unusable(UnusableTranslation::Reasoning))
    );
  }

  #[test]
  fn retry_stops_at_request_error_and_attempt_limit() {
    let (result, calls) =
      retry_with_outputs("Text", DESCRIPTION_ATTEMPTS, &[Err("network"), Ok("文本")]);
    assert_eq!(
      result,
      Err(TranslationFailure::Request("network".to_string()))
    );
    assert_eq!(calls, 1);

    let (result, calls) =
      retry_with_outputs("# Demo", BODY_ATTEMPTS, &[Ok("# Demo"), Ok("# 演示")]);
    assert_eq!(
      result,
      Err(TranslationFailure::Unusable(
        UnusableTranslation::Untranslated
      ))
    );
    assert_eq!(calls, 1);
  }

  #[test]
  fn lookup_ignores_cached_entry_with_unremovable_reasoning() {
    let dir = tempfile::tempdir().unwrap();
    let config = translate_config();
    let key = build_translation_cache_key("test/model", "Chinese", "Run the demo.");
    save_translation_cache_entry(dir.path(), &key, "Thinking Process:\n\n运行演示。").unwrap();

    assert_eq!(
      lookup_cached_translation(&config, dir.path(), "Run the demo."),
      None
    );
  }
}
