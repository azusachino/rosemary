use super::commands::SkillsCommands;
use super::runtime::*;
use crate::api::SkillStore;
use crate::paths::AsobiPaths;
use anyhow::Result;
use std::io::IsTerminal;
use tracing::{info, warn};

/// A source resolved to something installable.
struct Checkout {
    /// Directory holding the source's skill files.
    path: std::path::PathBuf,
    /// Version to record: the git commit, or `local` for a path source.
    version: String,
    /// The canonical source string to record on each skill.
    url: String,
}

/// Resolve a source to a directory holding its skills. Git sources go through
/// the shared clone cache; local paths are used in place.
fn checkout_source(source: &str, caches_dir: &std::path::Path) -> Result<Checkout> {
    let (url, is_git) = classify_skill_source(source);
    let (path, version) = if is_git {
        get_or_update_cached_repo(&url, caches_dir)?
    } else {
        let local_path = std::path::Path::new(source);
        if !local_path.exists() {
            anyhow::bail!("Local path {} does not exist", source);
        }
        (local_path.to_path_buf(), "local".to_string())
    };
    Ok(Checkout { path, version, url })
}

fn classify_skill_source(source: &str) -> (String, bool) {
    let mut git_url = source.to_string();
    let is_git = if source.contains("://") || source.contains("git@") {
        true
    } else if source.contains("github.com/") || source.contains("gitlab.com/") {
        git_url = format!("https://{source}");
        true
    } else {
        !std::path::Path::new(source).is_dir() && source.ends_with(".git")
    };
    (git_url, is_git)
}

pub(crate) fn run(
    backend: &crate::storage::Storage,
    paths: &AsobiPaths,
    subcommand: Option<SkillsCommands>,
) -> Result<()> {
    match subcommand {
        None => {
            let skills = backend.list_skills()?;
            if skills.is_empty() {
                println!("No skills installed.");
            } else {
                let mut grouped: std::collections::BTreeMap<String, Vec<crate::api::SkillRecord>> =
                    std::collections::BTreeMap::new();
                for s in skills {
                    grouped.entry(s.source.clone()).or_default().push(s);
                }
                println!("Installed Skills:");
                for (source, list) in grouped {
                    println!("Source: {}", source);
                    for s in list {
                        println!("  {} · {} · {}", s.entity_name, s.description, s.version);
                    }
                }
            }
        }
        Some(SkillsCommands::Install {
            source,
            all,
            select,
        }) => {
            let checkout = checkout_source(&source, &paths.caches_dir())?;

            let mode = if all {
                crate::skills::SelectionMode::All
            } else if let Some(sel) = select {
                crate::skills::SelectionMode::Select(sel)
            } else {
                crate::skills::SelectionMode::Interactive
            };

            let is_tty = std::io::stdin().is_terminal();

            // `--all` is a full sync of the source: prune skills that
            // vanished upstream. `--select` / interactive stay additive.
            let prune = matches!(mode, crate::skills::SelectionMode::All);

            crate::skills::install_skills_from_dir(
                backend,
                &checkout.path,
                &checkout.url,
                &checkout.version,
                mode,
                is_tty,
                prune,
            )?;

            info!("Skills installed successfully.");
        }
        Some(SkillsCommands::Sync) => {
            let config_file = paths.config_file.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "no asobi.toml found; `skills sync` reads its `[skills]` block \
                     (create one with `asobi init --local`)"
                )
            })?;
            let config =
                crate::skills_config::SkillsConfig::load(config_file)?.ok_or_else(|| {
                    anyhow::anyhow!("{} declares no `[skills]` block", config_file.display())
                })?;
            if config.sources.is_empty() {
                anyhow::bail!(
                    "{} declares no `[[skills.source]]` entries",
                    config_file.display()
                );
            }

            // Validate the whole declaration before touching anything, so a
            // typo in the last source does not leave a half-applied sync.
            let selections = config
                .sources
                .iter()
                .map(|s| s.selection())
                .collect::<Result<Vec<_>>>()?;

            let mut declared_slugs = std::collections::HashSet::new();
            let mut desired = Vec::new();
            for (declared, mode) in config.sources.iter().zip(selections) {
                let checkout = checkout_source(&declared.url, &paths.caches_dir())?;
                declared_slugs.insert(crate::skills::derive_source_slug(&checkout.url));

                let outcome = crate::skills::install_skills_from_dir(
                    backend,
                    &checkout.path,
                    &checkout.url,
                    &checkout.version,
                    mode,
                    false,
                    true,
                )?;
                info!(
                    "{}: {} installed, {} pruned",
                    declared.url,
                    outcome.installed.len(),
                    outcome.pruned.len()
                );
                desired.extend(outcome.installed);
            }

            // A source dropped from the config leaves the graph entirely.
            let stale: Vec<String> = backend
                .list_skills()?
                .into_iter()
                .filter(|s| !declared_slugs.contains(&crate::skills::derive_source_slug(&s.source)))
                .map(|s| s.entity_name)
                .collect();
            if !stale.is_empty() {
                info!("Removing {} skills from undeclared sources", stale.len());
                backend.remove_skills(stale)?;
            }

            let skills_dir = config.resolved_path(&paths.root);
            let written = crate::skills::materialize_skills(backend, &skills_dir, &desired)?;
            info!(
                "Synced {} skills into {} ({} written, {} removed)",
                desired.len(),
                skills_dir.display(),
                written.written.len(),
                written.removed.len()
            );
        }
        Some(SkillsCommands::Update { source }) => {
            let skills = backend.list_skills()?;
            let mut unique_sources = std::collections::HashSet::new();
            for s in skills {
                if let Some(ref filter_src) = source {
                    let slug = crate::skills::derive_source_slug(&s.source);
                    if &s.source == filter_src || &slug == filter_src {
                        unique_sources.insert(s.source.clone());
                    }
                } else {
                    unique_sources.insert(s.source.clone());
                }
            }

            if unique_sources.is_empty() {
                if let Some(src_val) = source {
                    anyhow::bail!(
                        "No installed skills found matching source/slug {:?}",
                        src_val
                    );
                } else {
                    info!("No skills currently installed.");
                    return Ok(());
                }
            }

            for src in unique_sources {
                info!("Updating skills from {}...", src);
                let (git_url, is_git) = classify_skill_source(&src);

                let (target_path, version) = if is_git {
                    let (cache_path, ver) =
                        get_or_update_cached_repo(&git_url, &paths.caches_dir())?;
                    (cache_path, ver)
                } else {
                    let local_path = std::path::Path::new(&src);
                    if !local_path.exists() {
                        warn!("Local path {} does not exist, skipping update", src);
                        continue;
                    }
                    (local_path.to_path_buf(), "local".to_string())
                };

                crate::skills::install_skills_from_dir(
                    backend,
                    &target_path,
                    &git_url,
                    &version,
                    crate::skills::SelectionMode::All,
                    false,
                    true,
                )?;
                info!("Successfully updated skills from {}.", src);
            }
        }
        Some(SkillsCommands::Remove { target }) => {
            let skills = backend.list_skills()?;
            let mut entities_to_delete = Vec::new();
            for s in skills {
                let slug = crate::skills::derive_source_slug(&s.source);
                if s.entity_name == target || s.source == target || slug == target {
                    entities_to_delete.push(s.entity_name.clone());
                }
            }

            if !entities_to_delete.is_empty() {
                info!("Deleting {} skill entities...", entities_to_delete.len());
                backend.remove_skills(entities_to_delete)?;
                info!("Skills removed successfully.");
            } else if target.starts_with("skill:") {
                info!("Deleting skill entity {}...", target);
                backend.remove_skills(vec![target.clone()])?;
                info!("Skills removed successfully.");
            } else {
                anyhow::bail!("No installed skills found matching target {:?}", target);
            }
        }
        Some(SkillsCommands::Show { name }) => {
            let mut entity_name = name.clone();
            if !entity_name.starts_with("skill:") {
                let skills = backend.list_skills()?;
                let matches: Vec<_> = skills
                    .iter()
                    .filter(|s| {
                        s.entity_name == name || s.entity_name.ends_with(&format!(":{}", name))
                    })
                    .collect();
                if matches.len() == 1 {
                    entity_name = matches[0].entity_name.clone();
                } else if matches.len() > 1 {
                    anyhow::bail!(
                        "Ambiguous skill name '{}'. Matches: {}",
                        name,
                        matches
                            .iter()
                            .map(|s| &s.entity_name)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                } else {
                    entity_name = format!("skill:{}", name);
                }
            }

            match backend.skill_body(&entity_name)? {
                Some(body) => {
                    print!("{}", body);
                }
                None => {
                    anyhow::bail!("Skill '{}' not found", name);
                }
            }
        }
    }

    Ok(())
}
