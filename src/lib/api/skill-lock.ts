/**
 * Skill Lock API
 *
 * 处理 .agents/.skill-lock.json 的读写操作
 * 用于跟踪已安装技能的元数据
 */

import { apiCall } from "./index";

// ============ Types ============

/**
 * 表示锁文件中的单个已安装技能条目
 */
export interface SkillLockEntry {
  /** 标准化的源标识符（例如 "owner/repo", "mintlify/bun.com"） */
  source: string;
  /** 提供者/源类型（例如 "github", "mintlify", "huggingface", "local"） */
  sourceType: string;
  /** 用于安装技能的原始 URL（用于重新获取更新） */
  sourceUrl: string;
  /** 源仓库内的子路径（如果适用） */
  skillPath?: string;
  /**
   * GitHub tree SHA for the entire skill folder.
   * 当技能文件夹中的任何文件发生变化时，此哈希值会改变。
   * 由 telemetry server 通过 GitHub Trees API 获取。
   */
  skillFolderHash?: string;
  /** 首次安装技能时的 ISO 时间戳 */
  installedAt: string;
  /** 最后更新技能时的 ISO 时间戳 */
  updatedAt: string;
}

/**
 * 技能锁文件的结构
 */
export interface SkillLockFile {
  /** 用于未来迁移的架构版本 */
  version: number;
  /** 技能名称到其锁条目的映射 */
  skills: Record<string, SkillLockEntry>;
}

// ============ API Functions ============

/**
 * 读取技能锁文件
 *
 * @returns 锁文件内容，如果文件不存在则返回空结构
 */
export async function readSkillLock(): Promise<SkillLockFile> {
  return apiCall<SkillLockFile>("read_skill_lock");
}

/**
 * 写入技能锁文件
 *
 * @param lock - 要写入的锁文件内容
 */
export async function writeSkillLock(lock: SkillLockFile): Promise<void> {
  return apiCall<void>("write_skill_lock", { lock });
}

/**
 * 添加或更新技能锁文件中的条目
 *
 * @param skillName - 技能名称
 * @param entry - 锁条目（不包含 installedAt 和 updatedAt，它们会自动设置）
 */
export async function addSkillToLock(
  skillName: string,
  entry: Omit<SkillLockEntry, "installedAt" | "updatedAt">
): Promise<void> {
  return apiCall<void>("add_skill_to_lock", { skillName, entry });
}

/**
 * 从锁文件中移除技能
 *
 * @param skillName - 要移除的技能名称
 * @returns 如果技能存在并被移除返回 true，否则返回 false
 */
export async function removeSkillFromLock(skillName: string): Promise<boolean> {
  return apiCall<boolean>("remove_skill_from_lock", { skillName });
}

/**
 * 从锁文件中获取单个技能条目
 *
 * @param skillName - 技能名称
 * @returns 技能条目，如果不存在则返回 null
 */
export async function getSkillFromLock(skillName: string): Promise<SkillLockEntry | null> {
  return apiCall<SkillLockEntry | null>("get_skill_from_lock", { skillName });
}

/**
 * 获取锁文件中的所有技能
 *
 * @returns 所有技能的映射表
 */
export async function getAllLockedSkills(): Promise<Record<string, SkillLockEntry>> {
  return apiCall<Record<string, SkillLockEntry>>("get_all_locked_skills");
}

/**
 * 按源分组获取技能（用于批量更新操作）
 *
 * @returns 按源分组的技能映射
 */
export async function getSkillsBySource(): Promise<
  Map<string, { skills: string[]; entry: SkillLockEntry }>
> {
  const lock = await readSkillLock();
  const bySource = new Map<string, { skills: string[]; entry: SkillLockEntry }>();

  for (const [skillName, entry] of Object.entries(lock.skills)) {
    const existing = bySource.get(entry.source);
    if (existing) {
      existing.skills.push(skillName);
    } else {
      bySource.set(entry.source, { skills: [skillName], entry });
    }
  }

  return bySource;
}
