use anyhow::{Result, anyhow, bail};
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionMode {
    All,
    Select(Vec<String>),
    Interactive,
}

/// What one source install changed in the graph.
#[derive(Debug, Default)]
pub struct InstallOutcome {
    /// Entity names installed or refreshed.
    pub installed: Vec<String>,
    /// Entity names dropped because the selection no longer covers them.
    pub pruned: Vec<String>,
}

/// What one sync changed on disk.
#[derive(Debug, Default)]
pub struct MaterializeOutcome {
    /// Skill directories created or rewritten.
    pub written: Vec<String>,
    /// Skill directories removed because the config no longer declares them.
    pub removed: Vec<String>,
}

pub fn parse_frontmatter(content: &str) -> Option<(Option<String>, Option<String>)> {
    let fm = crate::frontmatter::parse(content)?;
    Some((
        fm.get("name").map(str::to_string),
        fm.get("description").map(str::to_string),
    ))
}

pub fn derive_source_slug(url: &str) -> String {
    let mut trimmed = url.trim();
    if trimmed.ends_with(".git") {
        trimmed = &trimmed[..trimmed.len() - 4];
    }

    // Trim trailing slashes
    trimmed = trimmed.trim_end_matches('/');

    // 1. https/http URLs
    if let Some(pos) = trimmed.find("://") {
        let path = &trimmed[pos + 3..];
        if let Some(slash_pos) = path.find('/') {
            let parts: Vec<&str> = path[slash_pos + 1..].split('/').collect();
            if parts.len() >= 2 {
                return format!("{}-{}", parts[0], parts[1]);
            }
        }
    }

    // 2. SSH URL git@github.com:owner/repo
    if let Some(colon_pos) = trimmed.find(':') {
        let path = &trimmed[colon_pos + 1..];
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return format!("{}-{}", parts[0], parts[1]);
        }
    }

    // 3. Fallback: take last two components
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() >= 2 {
        let len = parts.len();
        return format!("{}-{}", parts[len - 2], parts[len - 1]);
    }

    crate::normalize::normalize_key(url)
}

/// When a skill file has no frontmatter `name:`, derive it from the filename
/// stem — except for convention filenames (`SKILL.md`, `index.md`), where the
/// skill's identity is the parent directory name.
fn resolve_skill_name_fallback(path: &Path) -> String {
    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if file_stem.eq_ignore_ascii_case("SKILL") || file_stem.eq_ignore_ascii_case("index") {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or(file_stem)
            .to_string()
    } else {
        file_stem.to_string()
    }
}

/// How many link hops [`inline_local_references`] follows before giving up.
/// Real skills reference at most one or two levels deep (`SKILL.md` ->
/// `reference.md`); this just bounds a pathological reference chain.
const MAX_REFERENCE_DEPTH: usize = 4;

/// Targets of every `[text](target)` inline link in `content`, in source
/// order and with duplicates kept (the caller dedupes via `visited`).
///
/// A hand-rolled scan rather than a markdown parser or a `regex` dependency:
/// skills only ever link with the plain inline form, and this only needs
/// candidate paths, not a rendered document. Reference-style links
/// (`[text][id]`) are deliberately not matched — rare in skills, and not
/// worth a second lookup table for.
fn markdown_link_targets(content: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    for (i, c) in content.char_indices() {
        if c != '[' {
            continue;
        }
        let Some(close_bracket) = content[i..].find(']') else {
            continue;
        };
        let after_bracket = i + close_bracket + 1;
        if !content[after_bracket..].starts_with('(') {
            continue;
        }
        let paren_start = after_bracket + 1;
        let Some(close_paren) = content[paren_start..].find(')') else {
            continue;
        };
        // A link target may carry a trailing `"title"`; only the path matters.
        let raw = content[paren_start..paren_start + close_paren].trim();
        let target = raw.split_whitespace().next().unwrap_or(raw);
        if !target.is_empty() {
            targets.push(target);
        }
    }
    targets
}

/// Inline-code-span targets in `content` that look like a path to a markdown
/// file: single backticks, no whitespace inside, ending in `.md`/`.markdown`.
/// Skips spans inside fenced (```` ``` ````) code blocks, since those are
/// examples to show the reader, not live references.
///
/// Some skills — Anthropic's own `skill-creator` among them — mention sibling
/// docs as prose ("see `references/schemas.md` for the schema") rather than
/// as a markdown link. The ending-in-`.md` + no-whitespace filter is loose by
/// itself (it would also match a *generated output* mentioned in backticks,
/// like "produces `benchmark.md`"), so the real filter is downstream: the
/// caller only inlines a target that resolves to a file that actually exists
/// in the source checkout, which a generated-output mention never does.
fn backtick_path_targets(content: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut in_fence = false;
    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        // Splitting on the delimiter puts every span *between* a pair of
        // backticks at an odd index; unmatched trailing backticks just leave
        // a final segment that is never treated as "inside".
        for (idx, span) in line.split('`').enumerate() {
            if idx % 2 == 0 {
                continue;
            }
            let looks_like_markdown_path = span.ends_with(".md") || span.ends_with(".markdown");
            if !span.is_empty() && !span.contains(char::is_whitespace) && looks_like_markdown_path {
                targets.push(span);
            }
        }
    }
    targets
}

/// True for a link target worth following on disk: not an external URL, mail
/// link, or a same-page anchor.
fn is_local_reference(target: &str) -> bool {
    !target.starts_with('#')
        && !target.contains("://")
        && !target.starts_with("mailto:")
        && !target.starts_with("tel:")
}

/// Inline the local `.md`/`.markdown` files a skill references — via a
/// markdown link or a backtick-quoted path — so a `SKILL.md` that is itself
/// just a table of contents over sibling docs ships as one self-contained
/// body. Asobi stores and materializes a single string per skill (see
/// [`materialize_skills`]), so anything the skill needs at runtime has to
/// live in that string — a reference file left beside `SKILL.md` in the
/// source repo never reaches `.agents/skills/<slug>@<name>/`.
///
/// Only references that resolve inside `root_dir` are followed (no escaping
/// the source checkout via `../..`), and a reference to another skill's own
/// entry point (`SKILL.md`/`index.md`) is never inlined — that is a
/// cross-skill reference to something installed as its own entity, not local
/// content. `depth` bounds how many reference hops are followed; `visited`
/// prevents cycles and re-inlining the same file reached two different ways.
fn inline_local_references(
    content: &str,
    file_dir: &Path,
    root_dir: &Path,
    visited: &mut std::collections::HashSet<std::path::PathBuf>,
    depth: usize,
) -> String {
    if depth == 0 {
        return content.to_string();
    }
    let mut out = content.to_string();
    let mut candidates = markdown_link_targets(content);
    candidates.extend(backtick_path_targets(content));
    for target in candidates {
        if !is_local_reference(target) {
            continue;
        }
        // Strip a `#fragment` before resolving; the file is what gets inlined.
        let relative = target.split('#').next().unwrap_or(target);
        let Ok(resolved) = file_dir.join(relative).canonicalize() else {
            continue;
        };
        if !resolved.starts_with(root_dir) {
            continue;
        }
        let is_markdown = resolved
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown")
            });
        let is_entry_point = resolved
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|stem| {
                stem.eq_ignore_ascii_case("SKILL") || stem.eq_ignore_ascii_case("index")
            });
        if !is_markdown || is_entry_point || !visited.insert(resolved.clone()) {
            continue;
        }
        let Ok(ref_content) = std::fs::read_to_string(&resolved) else {
            continue;
        };
        let label = resolved
            .strip_prefix(root_dir)
            .unwrap_or(&resolved)
            .display();
        out.push_str(&format!("\n\n---\n\n<!-- inlined: {label} -->\n\n"));
        let ref_dir = resolved.parent().unwrap_or(file_dir).to_path_buf();
        out.push_str(&inline_local_references(
            &ref_content,
            &ref_dir,
            root_dir,
            visited,
            depth - 1,
        ));
    }
    out
}

/// Entry point for [`inline_local_references`]: canonicalizes `root_dir` once
/// and seeds `visited` with the skill file's own path so a link back to
/// itself is a no-op rather than a duplicate of the whole body.
fn inline_references(content: &str, file_path: &Path, root_dir: &Path) -> String {
    let Some(file_dir) = file_path.parent() else {
        return content.to_string();
    };
    let Ok(root) = root_dir.canonicalize() else {
        return content.to_string();
    };
    let mut visited = std::collections::HashSet::new();
    if let Ok(canonical_self) = file_path.canonicalize() {
        visited.insert(canonical_self);
    }
    inline_local_references(content, file_dir, &root, &mut visited, MAX_REFERENCE_DEPTH)
}

pub fn resolve_selection(
    skills: &[(String, String)],
    mode: SelectionMode,
    is_tty: bool,
) -> Result<Vec<String>> {
    match mode {
        SelectionMode::All => Ok(skills.iter().map(|(n, _)| n.clone()).collect()),
        SelectionMode::Select(names) => {
            let mut selected = Vec::new();
            for name in names {
                if skills.iter().any(|(n, _)| n == &name) {
                    selected.push(name);
                } else {
                    bail!("Skill '{}' not found in source", name);
                }
            }
            Ok(selected)
        }
        SelectionMode::Interactive => {
            if !is_tty {
                bail!("Cannot resolve selection interactively: not a TTY. Use --all or --select");
            }
            use std::io::{self, Write};
            println!("Available skills:");
            for (i, (name, desc)) in skills.iter().enumerate() {
                println!("  [{}] {} - {}", i + 1, name, desc);
            }
            print!("Enter the numbers of the skills to install (comma-separated, e.g. 1, 3): ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let mut selected = Vec::new();
            for part in input.split(|c: char| c == ',' || c.is_whitespace()) {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                if let Ok(idx) = part.parse::<usize>() {
                    if idx > 0 && idx <= skills.len() {
                        selected.push(skills[idx - 1].0.clone());
                    } else {
                        bail!("Invalid skill index: {}", idx);
                    }
                } else {
                    bail!("Invalid input: {}", part);
                }
            }
            if selected.is_empty() {
                bail!("No skills selected");
            }
            Ok(selected)
        }
    }
}

pub fn install_skills_from_dir<S: crate::api::SkillStore>(
    store: &S,
    dir_path: &Path,
    source: &str,
    version: &str,
    mode: SelectionMode,
    is_tty: bool,
    prune: bool,
) -> Result<InstallOutcome> {
    let mut parsed_skills = Vec::new();
    let mut skill_contents = HashMap::new();
    // Every file path that claimed each name. Real-world skill repos sometimes
    // mirror the same skill under two tool-specific directories (e.g.
    // `.opencode/skills/x/` and `skills/x/`) and those mirrors can genuinely
    // diverge in content (different description, different frontmatter), so a
    // `HashMap<String, String>` silently keeping whichever file was walked
    // last -- and resolve_selection still returning the name twice, so the
    // second `.remove()` further down hit an already-emptied slot -- is not
    // safe. Collected here rather than checked eagerly, so a repo-wide name
    // collision only blocks installing *that* name: a narrow `--select` of an
    // unrelated, unambiguous skill in the same source still succeeds.
    let mut skill_paths: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();
    for entry in WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && e.path().extension().is_some_and(|ext| ext == "md"))
    {
        let content = std::fs::read_to_string(entry.path())?.replace("\r\n", "\n");
        if let Some((parsed_name, parsed_desc)) = parse_frontmatter(&content) {
            let name = parsed_name.unwrap_or_else(|| resolve_skill_name_fallback(entry.path()));
            skill_paths
                .entry(name.clone())
                .or_default()
                .push(entry.path().to_path_buf());
            let body = inline_references(&content, entry.path(), dir_path);
            parsed_skills.push((name.clone(), parsed_desc.unwrap_or_default()));
            skill_contents.insert(name, body);
        }
    }
    if parsed_skills.is_empty() {
        bail!("No valid skills found in {}", source);
    }
    let selected_names = resolve_selection(&parsed_skills, mode, is_tty)?;
    if let Some(name) = selected_names
        .iter()
        .find(|n| skill_paths.get(*n).is_some_and(|paths| paths.len() > 1))
    {
        let paths = skill_paths[name]
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "skill name '{}' is declared in more than one file in {}: {} -- \
             rename one so names stay unique",
            name,
            source,
            paths
        );
    }
    let slug = derive_source_slug(source);
    let mut outcome = InstallOutcome::default();
    if prune {
        let fresh: std::collections::HashSet<String> = selected_names
            .iter()
            .map(|n| crate::normalize::normalize_key(&format!("skill:{}:{}", slug, n)))
            .collect();
        let orphans = store
            .list_skills()?
            .into_iter()
            .filter(|s| derive_source_slug(&s.source) == slug && !fresh.contains(&s.entity_name))
            .map(|s| s.entity_name)
            .collect::<Vec<_>>();
        if !orphans.is_empty() {
            store.remove_skills(orphans.clone())?;
            outcome.pruned = orphans;
        }
    }
    for name in selected_names {
        let body = skill_contents
            .remove(&name)
            .ok_or_else(|| anyhow!("Content missing for skill {}", name))?;
        let description = parsed_skills
            .iter()
            .find(|(n, _)| n == &name)
            .map(|(_, d)| d.clone())
            .unwrap_or_default();
        let entity_name = crate::normalize::normalize_key(&format!("skill:{}:{}", slug, name));
        store.upsert_skill(crate::api::SkillRecord {
            entity_name: entity_name.clone(),
            body,
            source: source.to_string(),
            version: version.to_string(),
            description,
        })?;
        outcome.installed.push(entity_name);
    }
    Ok(outcome)
}

/// The on-disk directory for a skill entity: `skill:<slug>:<name>` becomes
/// `<slug>@<name>`. Returns `None` for anything that is not a skill entity.
///
/// Both halves are slugified to lowercase kebab-case, so a frontmatter name
/// like `Verification Before Completion` lands in a canonical directory rather
/// than carrying its display capitalisation onto the filesystem.
pub fn skill_dir_name(entity_name: &str) -> Option<String> {
    let rest = entity_name.strip_prefix("skill:")?;
    let (slug, name) = rest.split_once(':')?;
    if slug.is_empty() || name.is_empty() {
        return None;
    }
    Some(format!(
        "{}@{}",
        crate::normalize::slugify(slug),
        crate::normalize::slugify(name)
    ))
}

/// Write `desired` out as `<dir>/<slug>@<name>/SKILL.md`, then remove every
/// other `<slug>@<name>` directory under `dir`.
///
/// Pruning is deliberately scoped to the `@` naming convention: directories
/// Asobi did not write — a hand-authored skill, a vendored upstream checkout —
/// have no `@` in their name and are never touched.
pub fn materialize_skills<S: crate::api::SkillStore>(
    store: &S,
    dir: &Path,
    desired: &[String],
) -> Result<MaterializeOutcome> {
    let mut outcome = MaterializeOutcome::default();
    let wanted: std::collections::HashSet<String> =
        desired.iter().filter_map(|e| skill_dir_name(e)).collect();

    std::fs::create_dir_all(dir)?;

    for entity in desired {
        let Some(dir_name) = skill_dir_name(entity) else {
            continue;
        };
        let body = store
            .skill_body(entity)?
            .ok_or_else(|| anyhow!("Skill '{}' has no body to write", entity))?;
        let skill_dir = dir.join(&dir_name);
        let file = skill_dir.join("SKILL.md");
        // Leave an already-current file alone so a no-op sync does not churn
        // mtimes that file watchers key off.
        if std::fs::read_to_string(&file).is_ok_and(|existing| existing == body) {
            continue;
        }
        std::fs::create_dir_all(&skill_dir)?;
        std::fs::write(&file, body)?;
        outcome.written.push(dir_name);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if name.contains('@') && !wanted.contains(&name) {
            std::fs::remove_dir_all(entry.path())?;
            outcome.removed.push(name);
        }
    }

    outcome.written.sort();
    outcome.removed.sort();
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::SkillStore;
    use crate::storage::Storage;

    #[test]
    fn test_parse_frontmatter_valid() {
        let content = "---\nname: my-skill\ndescription: \"does something\"\n---\nbody content";
        let parsed = parse_frontmatter(content);
        assert_eq!(
            parsed,
            Some((
                Some("my-skill".to_string()),
                Some("does something".to_string())
            ))
        );
    }

    #[test]
    fn test_parse_frontmatter_missing() {
        let content = "no frontmatter here";
        let parsed = parse_frontmatter(content);
        assert_eq!(parsed, None);
    }

    #[test]
    fn test_parse_frontmatter_malformed() {
        let content = "---\nname: partial-skill\n---\nbody content";
        let parsed = parse_frontmatter(content);
        assert_eq!(parsed, Some((Some("partial-skill".to_string()), None)));
    }

    #[test]
    fn test_derive_source_slug() {
        assert_eq!(
            derive_source_slug("https://github.com/jasonswett/llm-skills.git"),
            "jasonswett-llm-skills"
        );
        assert_eq!(
            derive_source_slug("git@github.com:jasonswett/llm-skills.git"),
            "jasonswett-llm-skills"
        );
        assert_eq!(
            derive_source_slug("/path/to/local-skills"),
            "to-local-skills"
        );
    }

    #[test]
    fn test_resolve_selection_all() {
        let skills = vec![
            ("skill-a".to_string(), "desc-a".to_string()),
            ("skill-b".to_string(), "desc-b".to_string()),
        ];
        let selected = resolve_selection(&skills, SelectionMode::All, false).unwrap();
        assert_eq!(selected, vec!["skill-a", "skill-b"]);
    }

    #[test]
    fn test_resolve_selection_select() {
        let skills = vec![
            ("skill-a".to_string(), "desc-a".to_string()),
            ("skill-b".to_string(), "desc-b".to_string()),
        ];
        let selected = resolve_selection(
            &skills,
            SelectionMode::Select(vec!["skill-b".to_string()]),
            false,
        )
        .unwrap();
        assert_eq!(selected, vec!["skill-b"]);
    }

    #[test]
    fn test_install_from_local_git_repo() {
        use tempfile::tempdir;
        let git_dir = tempdir().unwrap();
        let repo_path = git_dir.path();

        // 1. Initialize git repo
        std::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .status()
            .unwrap();

        // Set git config for local commit
        std::process::Command::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test User")
            .current_dir(repo_path)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(repo_path)
            .status()
            .unwrap();

        // 2. Create a skill file
        let skill_file = repo_path.join("test-skill.md");
        std::fs::write(
            &skill_file,
            "---\nname: repo-skill\ndescription: cloned skill\n---\nbody text\n",
        )
        .unwrap();

        // 3. Commit the file
        std::process::Command::new("git")
            .arg("add")
            .arg("test-skill.md")
            .current_dir(repo_path)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("initial commit")
            .current_dir(repo_path)
            .status()
            .unwrap();

        // Get HEAD commit hash
        let output = std::process::Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(repo_path)
            .output()
            .unwrap();
        let head_commit = String::from_utf8(output.stdout).unwrap().trim().to_string();

        // 4. Setup temp database
        let db_dir = tempdir().unwrap();
        unsafe {
            std::env::set_var(
                crate::paths::ENV_DATABASE_URL,
                db_dir.path().join("test.db").to_str().unwrap(),
            );
        }
        let storage = Storage::open_default().unwrap();

        // 5. Clone and install
        let clone_temp_dir = tempdir().unwrap();
        let clone_path = clone_temp_dir.path();
        std::process::Command::new("git")
            .arg("clone")
            .arg(repo_path.to_str().unwrap())
            .arg(clone_path.to_str().unwrap())
            .status()
            .unwrap();

        install_skills_from_dir(
            &storage,
            clone_path,
            repo_path.to_str().unwrap(),
            &head_commit,
            SelectionMode::All,
            false,
            true,
        )
        .unwrap();

        // 6. Verify skill installed
        let skills = storage.list_skills().unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(
            skills[0].entity_name,
            crate::normalize::normalize_key(&format!(
                "skill:{}:repo-skill",
                derive_source_slug(repo_path.to_str().unwrap())
            ))
        );
        assert_eq!(skills[0].version, head_commit);
    }

    #[test]
    fn test_sync_prunes_orphaned_skills() {
        use tempfile::tempdir;
        let src_dir = tempdir().unwrap();
        let src = src_dir.path();

        // Initial source with two skills.
        std::fs::write(
            src.join("alpha.md"),
            "---\nname: alpha\ndescription: a\n---\nalpha body\n",
        )
        .unwrap();
        std::fs::write(
            src.join("beta.md"),
            "---\nname: beta\ndescription: b\n---\nbeta body\n",
        )
        .unwrap();

        let db_dir = tempdir().unwrap();
        unsafe {
            std::env::set_var(
                crate::paths::ENV_DATABASE_URL,
                db_dir.path().join("test.db").to_str().unwrap(),
            );
        }
        let storage = Storage::open_default().unwrap();

        let source = src.to_str().unwrap();
        let slug = derive_source_slug(source);

        install_skills_from_dir(&storage, src, source, "v1", SelectionMode::All, false, true)
            .unwrap();
        assert_eq!(storage.list_skills().unwrap().len(), 2);

        // Upstream removes `beta`; a sync (install --all) must prune it.
        std::fs::remove_file(src.join("beta.md")).unwrap();

        install_skills_from_dir(&storage, src, source, "v2", SelectionMode::All, false, true)
            .unwrap();

        let skills = storage.list_skills().unwrap();
        assert_eq!(skills.len(), 1);
        let alpha = crate::normalize::normalize_key(&format!("skill:{}:alpha", slug));
        assert_eq!(skills[0].entity_name, alpha);
        assert_eq!(skills[0].version, "v2");
    }

    #[test]
    fn test_select_does_not_prune() {
        use tempfile::tempdir;
        let src_dir = tempdir().unwrap();
        let src = src_dir.path();

        std::fs::write(
            src.join("alpha.md"),
            "---\nname: alpha\ndescription: a\n---\nalpha body\n",
        )
        .unwrap();
        std::fs::write(
            src.join("beta.md"),
            "---\nname: beta\ndescription: b\n---\nbeta body\n",
        )
        .unwrap();

        let db_dir = tempdir().unwrap();
        unsafe {
            std::env::set_var(
                crate::paths::ENV_DATABASE_URL,
                db_dir.path().join("test.db").to_str().unwrap(),
            );
        }
        let storage = Storage::open_default().unwrap();
        let source = src.to_str().unwrap();

        // Install only alpha, then only beta — both must survive (additive).
        for name in ["alpha", "beta"] {
            install_skills_from_dir(
                &storage,
                src,
                source,
                "v1",
                SelectionMode::Select(vec![name.to_string()]),
                false,
                false,
            )
            .unwrap();
        }

        assert_eq!(storage.list_skills().unwrap().len(), 2);
    }

    #[test]
    fn test_skill_dir_name() {
        assert_eq!(
            skill_dir_name("skill:obra-superpowers:writing-plans").as_deref(),
            Some("obra-superpowers@writing-plans")
        );
        // A display-cased frontmatter name is canonicalised, not carried through.
        assert_eq!(
            skill_dir_name("skill:obra-superpowers:Verification-Before-Completion").as_deref(),
            Some("obra-superpowers@verification-before-completion")
        );
        assert_eq!(skill_dir_name("obra-superpowers:writing-plans"), None);
        assert_eq!(skill_dir_name("skill:no-name-part"), None);
    }

    #[test]
    fn test_materialize_writes_and_prunes_only_owned_dirs() {
        use tempfile::tempdir;
        let src_dir = tempdir().unwrap();
        let src = src_dir.path();
        std::fs::write(
            src.join("alpha.md"),
            "---\nname: alpha\ndescription: a\n---\nalpha body\n",
        )
        .unwrap();
        std::fs::write(
            src.join("beta.md"),
            "---\nname: beta\ndescription: b\n---\nbeta body\n",
        )
        .unwrap();

        let db_dir = tempdir().unwrap();
        unsafe {
            std::env::set_var(
                crate::paths::ENV_DATABASE_URL,
                db_dir.path().join("test.db").to_str().unwrap(),
            );
        }
        let storage = Storage::open_default().unwrap();
        let source = src.to_str().unwrap();

        let outcome =
            install_skills_from_dir(&storage, src, source, "v1", SelectionMode::All, false, true)
                .unwrap();
        assert_eq!(outcome.installed.len(), 2);

        let entity = |suffix: &str| -> String {
            outcome
                .installed
                .iter()
                .find(|e| e.ends_with(suffix))
                .unwrap()
                .clone()
        };
        let alpha_dir = skill_dir_name(&entity(":alpha")).unwrap();
        let beta_dir = skill_dir_name(&entity(":beta")).unwrap();

        let out_dir = tempdir().unwrap();
        let out = out_dir.path();
        // A vendored checkout and a hand-written note: neither has an `@`, so
        // neither is ours to delete.
        std::fs::create_dir(out.join("vendored-upstream")).unwrap();
        std::fs::write(out.join("README.md"), "mine").unwrap();

        let written = materialize_skills(&storage, out, &outcome.installed).unwrap();
        assert_eq!(written.written.len(), 2);
        assert!(written.removed.is_empty());
        assert_eq!(
            std::fs::read_to_string(out.join(&alpha_dir).join("SKILL.md")).unwrap(),
            "---\nname: alpha\ndescription: a\n---\nalpha body\n"
        );

        // Re-running with the same desired set is a no-op on disk.
        let again = materialize_skills(&storage, out, &outcome.installed).unwrap();
        assert!(again.written.is_empty());
        assert!(again.removed.is_empty());

        // Narrowing the desired set removes only the dropped skill.
        let narrowed = materialize_skills(&storage, out, &[entity(":alpha")]).unwrap();
        assert_eq!(narrowed.removed, vec![beta_dir.clone()]);
        assert!(out.join(&alpha_dir).is_dir());
        assert!(!out.join(&beta_dir).exists());
        assert!(out.join("vendored-upstream").is_dir());
        assert!(out.join("README.md").is_file());
    }

    #[test]
    fn test_install_skills_with_fallbacks() {
        use tempfile::tempdir;
        let git_dir = tempdir().unwrap();
        let repo_path = git_dir.path();

        // 1. Initialize git repo
        std::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .status()
            .unwrap();

        // Set git config for local commit
        std::process::Command::new("git")
            .arg("config")
            .arg("user.name")
            .arg("Test User")
            .current_dir(repo_path)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .current_dir(repo_path)
            .status()
            .unwrap();

        // 2. Create skill files with missing name and description respectively
        let refactor_file = repo_path.join("refactor.md");
        std::fs::write(
            &refactor_file,
            "---\ndescription: Iterative refactoring loop\n---\nrefactor body\n",
        )
        .unwrap();

        let sdr_dir = repo_path.join("software-design-review");
        std::fs::create_dir(&sdr_dir).unwrap();
        let sdr_file = sdr_dir.join("SKILL.md");
        std::fs::write(
            &sdr_file,
            "---\nname: software-design-review\n---\nsdr body\n",
        )
        .unwrap();

        // 3. Commit files
        std::process::Command::new("git")
            .arg("add")
            .arg("refactor.md")
            .arg("software-design-review/SKILL.md")
            .current_dir(repo_path)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("add skills")
            .current_dir(repo_path)
            .status()
            .unwrap();

        // Get HEAD commit hash
        let output = std::process::Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(repo_path)
            .output()
            .unwrap();
        let head_commit = String::from_utf8(output.stdout).unwrap().trim().to_string();

        // 4. Setup temp database
        let db_dir = tempdir().unwrap();
        unsafe {
            std::env::set_var(
                crate::paths::ENV_DATABASE_URL,
                db_dir.path().join("test.db").to_str().unwrap(),
            );
        }
        let storage = Storage::open_default().unwrap();

        // 5. Clone and install
        let clone_temp_dir = tempdir().unwrap();
        let clone_path = clone_temp_dir.path();
        std::process::Command::new("git")
            .arg("clone")
            .arg(repo_path.to_str().unwrap())
            .arg(clone_path.to_str().unwrap())
            .status()
            .unwrap();

        install_skills_from_dir(
            &storage,
            clone_path,
            repo_path.to_str().unwrap(),
            &head_commit,
            SelectionMode::All,
            false,
            true,
        )
        .unwrap();

        // 6. Verify skills installed correctly with fallbacks
        let skills = storage.list_skills().unwrap();
        assert_eq!(skills.len(), 2);

        let slug = derive_source_slug(repo_path.to_str().unwrap());

        let refactor_entity = crate::normalize::normalize_key(&format!("skill:{}:refactor", slug));
        let sdr_entity =
            crate::normalize::normalize_key(&format!("skill:{}:software-design-review", slug));

        let refactor_row = skills
            .iter()
            .find(|s| s.entity_name == refactor_entity)
            .unwrap();
        assert_eq!(refactor_row.description, "Iterative refactoring loop");

        let sdr_row = skills.iter().find(|s| s.entity_name == sdr_entity).unwrap();
        assert_eq!(sdr_row.description, "");
    }

    #[test]
    fn test_markdown_link_targets() {
        let content = "See [AGENT-BRIEF.md](AGENT-BRIEF.md) and \
             [a titled link](path/to.md \"a title\") and [anchor](#section) and \
             [site](https://example.com) plain text with no link.";
        assert_eq!(
            markdown_link_targets(content),
            vec![
                "AGENT-BRIEF.md",
                "path/to.md",
                "#section",
                "https://example.com"
            ]
        );
    }

    #[test]
    fn test_backtick_path_targets() {
        let content = "See `references/schemas.md` for the schema. Also `grading.json` \
             and `evidence` are just field names, and `path with space.md` doesn't count.\n\
             ```\n\
             this fenced `inside.md` mention must not match\n\
             ```\n\
             But `after-fence.md` on a normal line does.";
        assert_eq!(
            backtick_path_targets(content),
            vec!["references/schemas.md", "after-fence.md"]
        );
    }

    #[test]
    fn test_is_local_reference() {
        assert!(is_local_reference("reference.md"));
        assert!(is_local_reference("../sibling/reference.md"));
        assert!(!is_local_reference("#section"));
        assert!(!is_local_reference("https://example.com/doc.md"));
        assert!(!is_local_reference("mailto:you@example.com"));
    }

    #[test]
    fn test_inline_references_pulls_in_sibling_docs() {
        use tempfile::tempdir;
        let root = tempdir().unwrap();
        let skill_dir = root.path().join("triage");
        std::fs::create_dir(&skill_dir).unwrap();

        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: triage\ndescription: toc-style skill\n---\n\
             # Triage\n\n## Reference docs\n\n\
             - [AGENT-BRIEF.md](AGENT-BRIEF.md) -- how to write agent briefs\n\
             - [OUT-OF-SCOPE.md](OUT-OF-SCOPE.md) -- rejected work log\n\n\
             Later: post an agent brief ([AGENT-BRIEF.md](AGENT-BRIEF.md)).\n",
        )
        .unwrap();
        std::fs::write(
            skill_dir.join("AGENT-BRIEF.md"),
            "# Agent Brief\n\nWrite briefs like this.\n",
        )
        .unwrap();
        std::fs::write(
            skill_dir.join("OUT-OF-SCOPE.md"),
            "# Out of Scope\n\nRejected requests live here.\n",
        )
        .unwrap();

        let content = std::fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        let inlined = inline_references(&content, &skill_dir.join("SKILL.md"), root.path());

        assert!(inlined.contains("Write briefs like this."));
        assert!(inlined.contains("Rejected requests live here."));
        // Referenced twice in the source; inlined once (`visited` dedupes).
        assert_eq!(inlined.matches("Write briefs like this.").count(), 1);
    }

    #[test]
    fn test_inline_references_pulls_in_backtick_referenced_docs() {
        // Mirrors Anthropic's own skill-creator, which mentions references as
        // prose ("see `references/schemas.md`") rather than markdown links.
        use tempfile::tempdir;
        let root = tempdir().unwrap();
        let skill_dir = root.path().join("skill-creator");
        std::fs::create_dir(&skill_dir).unwrap();
        std::fs::create_dir(skill_dir.join("references")).unwrap();

        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: skill-creator\ndescription: toc-style skill\n---\n\
             See `references/schemas.md` for the full schema. This step \
             produces `benchmark.md`, which is not a file in this repo.\n",
        )
        .unwrap();
        std::fs::write(
            skill_dir.join("references/schemas.md"),
            "# Schemas\n\nThe JSON structures live here.\n",
        )
        .unwrap();

        let content = std::fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        let inlined = inline_references(&content, &skill_dir.join("SKILL.md"), root.path());

        assert!(inlined.contains("The JSON structures live here."));
        // `benchmark.md` is path-shaped but never exists on disk -- silent no-op.
        assert!(!inlined.contains("inlined: skill-creator/benchmark.md"));
    }

    #[test]
    fn test_inline_references_skips_path_traversal() {
        use tempfile::tempdir;
        let outer = tempdir().unwrap();
        let root = outer.path().join("root");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(outer.path().join("secret.md"), "outside content").unwrap();

        let skill_file = root.join("SKILL.md");
        std::fs::write(
            &skill_file,
            "---\nname: x\n---\nSee [secret](../secret.md).\n",
        )
        .unwrap();

        let content = std::fs::read_to_string(&skill_file).unwrap();
        let inlined = inline_references(&content, &skill_file, &root);
        assert!(!inlined.contains("outside content"));
    }

    #[test]
    fn test_inline_references_skips_other_skill_entry_points() {
        use tempfile::tempdir;
        let root = tempdir().unwrap();
        let a_dir = root.path().join("skill-a");
        let b_dir = root.path().join("skill-b");
        std::fs::create_dir(&a_dir).unwrap();
        std::fs::create_dir(&b_dir).unwrap();

        std::fs::write(
            b_dir.join("SKILL.md"),
            "---\nname: skill-b\n---\nskill b's own body\n",
        )
        .unwrap();
        let a_file = a_dir.join("SKILL.md");
        std::fs::write(
            &a_file,
            "---\nname: skill-a\n---\nSee also [skill-b](../skill-b/SKILL.md).\n",
        )
        .unwrap();

        let content = std::fs::read_to_string(&a_file).unwrap();
        let inlined = inline_references(&content, &a_file, root.path());
        assert!(!inlined.contains("skill b's own body"));
    }

    #[test]
    fn test_install_from_dir_inlines_sibling_references() {
        use tempfile::tempdir;
        let src_dir = tempdir().unwrap();
        let src = src_dir.path();

        std::fs::write(
            src.join("SKILL.md"),
            "---\nname: triage\ndescription: toc-style skill\n---\n\
             See [AGENT-BRIEF.md](AGENT-BRIEF.md) for the brief format.\n",
        )
        .unwrap();
        std::fs::write(
            src.join("AGENT-BRIEF.md"),
            "# Agent Brief\n\nWrite briefs like this.\n",
        )
        .unwrap();

        let db_dir = tempdir().unwrap();
        unsafe {
            std::env::set_var(
                crate::paths::ENV_DATABASE_URL,
                db_dir.path().join("test.db").to_str().unwrap(),
            );
        }
        let storage = Storage::open_default().unwrap();
        let source = src.to_str().unwrap();

        let outcome =
            install_skills_from_dir(&storage, src, source, "v1", SelectionMode::All, false, true)
                .unwrap();
        assert_eq!(outcome.installed.len(), 1);

        let body = storage.skill_body(&outcome.installed[0]).unwrap().unwrap();
        assert!(body.contains("Write briefs like this."));
    }

    #[test]
    fn test_install_rejects_duplicate_skill_names() {
        use tempfile::tempdir;
        let src_dir = tempdir().unwrap();
        let src = src_dir.path();

        // Mirrors a real pattern: the same skill name declared under two
        // tool-specific directories, with genuinely different descriptions --
        // picking one silently would drop the other's content.
        std::fs::create_dir(src.join("skills")).unwrap();
        std::fs::create_dir(src.join(".opencode")).unwrap();
        std::fs::write(
            src.join("skills/one.md"),
            "---\nname: dup\ndescription: canonical copy\n---\nbody one\n",
        )
        .unwrap();
        std::fs::write(
            src.join(".opencode/two.md"),
            "---\nname: dup\ndescription: mirrored copy\n---\nbody two\n",
        )
        .unwrap();

        let db_dir = tempdir().unwrap();
        unsafe {
            std::env::set_var(
                crate::paths::ENV_DATABASE_URL,
                db_dir.path().join("test.db").to_str().unwrap(),
            );
        }
        let storage = Storage::open_default().unwrap();
        let source = src.to_str().unwrap();

        let err =
            install_skills_from_dir(&storage, src, source, "v1", SelectionMode::All, false, true)
                .unwrap_err();
        let message = err.to_string();
        assert!(message.contains("dup"), "message was: {message}");
        assert!(message.contains("one.md"), "message was: {message}");
        assert!(message.contains("two.md"), "message was: {message}");
    }

    #[test]
    fn test_install_select_ignores_unrelated_duplicate_names() {
        use tempfile::tempdir;
        let src_dir = tempdir().unwrap();
        let src = src_dir.path();

        // Same collision as above, plus one unambiguous skill elsewhere in the
        // same source. A narrow `--select` of the unambiguous one must still
        // succeed -- the collision only matters for the name it actually blocks.
        std::fs::create_dir(src.join("skills")).unwrap();
        std::fs::create_dir(src.join(".opencode")).unwrap();
        std::fs::write(
            src.join("skills/one.md"),
            "---\nname: dup\ndescription: canonical copy\n---\nbody one\n",
        )
        .unwrap();
        std::fs::write(
            src.join(".opencode/two.md"),
            "---\nname: dup\ndescription: mirrored copy\n---\nbody two\n",
        )
        .unwrap();
        std::fs::write(
            src.join("fine.md"),
            "---\nname: fine\ndescription: no conflict\n---\nfine body\n",
        )
        .unwrap();

        let db_dir = tempdir().unwrap();
        unsafe {
            std::env::set_var(
                crate::paths::ENV_DATABASE_URL,
                db_dir.path().join("test.db").to_str().unwrap(),
            );
        }
        let storage = Storage::open_default().unwrap();
        let source = src.to_str().unwrap();

        let outcome = install_skills_from_dir(
            &storage,
            src,
            source,
            "v1",
            SelectionMode::Select(vec!["fine".to_string()]),
            false,
            false,
        )
        .unwrap();
        assert_eq!(outcome.installed.len(), 1);
        assert!(outcome.installed[0].ends_with(":fine"));
    }
}
