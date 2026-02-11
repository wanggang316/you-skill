#[derive(Debug, Clone)]
pub struct AgentPath {
  pub id: &'static str,
  pub display_name: &'static str,
  pub project_path: Option<&'static str>,
  pub global_path: Option<&'static str>,
}

pub fn agent_paths() -> Vec<AgentPath> {
  vec![
    AgentPath {
      id: "claude-code",
      display_name: "Claude Code",
      project_path: Some(".claude/skills"),
      global_path: Some("~/.claude/skills"),
    },
    AgentPath {
      id: "codex",
      display_name: "Codex",
      project_path: Some(".codex/skills"),
      global_path: Some("~/.codex/skills"),
    },
    AgentPath {
      id: "cursor",
      display_name: "Cursor",
      project_path: Some(".cursor/skills"),
      global_path: Some("~/.cursor/skills"),
    },
    AgentPath {
      id: "cline",
      display_name: "Cline",
      project_path: Some(".cline/skills"),
      global_path: Some("~/.cline/skills"),
    },
    AgentPath {
      id: "opencode",
      display_name: "OpenCode",
      project_path: Some(".opencode/skills"),
      global_path: Some("~/.config/opencode/skills"),
    },
    AgentPath {
      id: "openhands",
      display_name: "OpenHands",
      project_path: Some(".openhands/skills"),
      global_path: Some("~/.openhands/skills"),
    },
    AgentPath {
      id: "github-copilot",
      display_name: "GitHub Copilot",
      project_path: Some(".github/skills"),
      global_path: Some("~/.copilot/skills"),
    },
    AgentPath {
      id: "continue",
      display_name: "Continue",
      project_path: Some(".continue/skills"),
      global_path: Some("~/.continue/skills"),
    },
    AgentPath {
      id: "gemini-cli",
      display_name: "Gemini CLI",
      project_path: Some(".gemini/skills"),
      global_path: Some("~/.gemini/skills"),
    },
    AgentPath {
      id: "goose",
      display_name: "Goose",
      project_path: Some(".goose/skills"),
      global_path: Some("~/.config/goose/skills"),
    },
    AgentPath {
      id: "windsurf",
      display_name: "Windsurf",
      project_path: Some(".windsurf/skills"),
      global_path: Some("~/.codeium/windsurf/skills"),
    },
    AgentPath {
      id: "roo",
      display_name: "Roo Code",
      project_path: Some(".roo/skills"),
      global_path: Some("~/.roo/skills"),
    },
    AgentPath {
      id: "kiro-cli",
      display_name: "Kiro CLI",
      project_path: Some(".kiro/skills"),
      global_path: Some("~/.kiro/skills"),
    },
    AgentPath {
      id: "qwen-code",
      display_name: "Qwen Code",
      project_path: Some(".qwen/skills"),
      global_path: Some("~/.qwen/skills"),
    },
    AgentPath {
      id: "amp",
      display_name: "AMP",
      project_path: Some(".agents/skills"),
      global_path: Some("~/.config/agents/skills"),
    },
    AgentPath {
      id: "antigravity",
      display_name: "Antigravity",
      project_path: Some(".agent/skills"),
      global_path: Some("~/.gemini/antigravity/skills"),
    },
    AgentPath {
      id: "command-code",
      display_name: "Command Code",
      project_path: Some(".commandcode/skills"),
      global_path: Some("~/.commandcode/skills"),
    },
    AgentPath {
      id: "crush",
      display_name: "Crush",
      project_path: Some(".crush/skills"),
      global_path: Some("~/.config/crush/skills"),
    },
    AgentPath {
      id: "trae",
      display_name: "Trae",
      project_path: Some(".trae/skills"),
      global_path: Some("~/.trae/skills"),
    },
    AgentPath {
      id: "trae-cn",
      display_name: "Trae CN",
      project_path: Some(".trae-cn/skills"),
      global_path: Some("~/.trae-cn/skills"),
    },
    AgentPath {
      id: "vscode",
      display_name: "VSCode",
      project_path: Some(".github/skills"),
      global_path: Some("~/.vscode/skills"),
    },
  ]
}
