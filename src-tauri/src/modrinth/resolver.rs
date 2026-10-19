use super::client::{ModrinthClient, ProjectVersion};
use crate::error::CommandError;
use std::collections::{HashMap, HashSet, VecDeque};

const MAX_RESOLUTION_PASSES: usize = 16;

pub async fn resolve(
    client: &ModrinthClient,
    roots: &[String],
    game: &str,
    loader: &str,
    pinned_versions: &HashMap<String, String>,
) -> Result<Vec<ProjectVersion>, CommandError> {
    let mut exact_versions = pinned_versions.clone();
    for _ in 0..MAX_RESOLUTION_PASSES {
        let (resolved, discovered_exact_versions) = resolve_pass(
            client,
            roots,
            game,
            loader,
            &exact_versions,
            pinned_versions,
        )
        .await?;
        if discovered_exact_versions == exact_versions {
            validate_incompatible_dependencies(&resolved)?;
            let mut versions: Vec<_> = resolved.into_values().collect();
            versions.sort_by(|left, right| left.project_id.cmp(&right.project_id));
            return Ok(versions);
        }
        exact_versions = discovered_exact_versions;
    }
    Err(CommandError::new(
        "The selected mods contain dependency requirements that could not be stabilized.",
    ))
}

fn validate_incompatible_dependencies(
    resolved: &HashMap<String, ProjectVersion>,
) -> Result<(), CommandError> {
    for version in resolved.values() {
        for dependency in &version.dependencies {
            if dependency.dependency_type != "incompatible" {
                continue;
            }
            let Some(project_id) = &dependency.project_id else {
                continue;
            };
            let Some(conflicting) = resolved.get(project_id) else {
                continue;
            };
            if dependency
                .version_id
                .as_ref()
                .is_some_and(|required_id| required_id != &conflicting.id)
            {
                continue;
            }
            return Err(CommandError::new(format!(
                "Mod {} ({}) is incompatible with mod {} ({}). Select another version or disable one of these mods.",
                version.name, version.project_id, conflicting.name, conflicting.project_id
            )));
        }
    }
    Ok(())
}

async fn resolve_pass(
    client: &ModrinthClient,
    roots: &[String],
    game: &str,
    loader: &str,
    exact_versions: &HashMap<String, String>,
    pinned_versions: &HashMap<String, String>,
) -> Result<(HashMap<String, ProjectVersion>, HashMap<String, String>), CommandError> {
    let mut queue: VecDeque<String> = roots.iter().cloned().collect();
    let mut visited = HashSet::new();
    let mut resolved = HashMap::new();
    let mut discovered_exact_versions = pinned_versions.clone();

    while let Some(project_id) = queue.pop_front() {
        if !visited.insert(project_id.clone()) {
            continue;
        }
        let exact_version = exact_versions
            .get(&project_id)
            .or_else(|| discovered_exact_versions.get(&project_id));
        let version = if let Some(version_id) = exact_version {
            let version = client.version(version_id).await?;
            validate_exact_version(&version, &project_id, game, loader)?;
            version
        } else {
            let versions = client.versions(&project_id, game, loader).await?;
            match select_preferred(versions) {
                Some(version) => version,
                None => return Err(incompatible_project_error(client, &project_id).await),
            }
        };

        for dependency in &version.dependencies {
            if dependency.dependency_type != "required" {
                continue;
            }
            let dependency_project = if let Some(project_id) = &dependency.project_id {
                project_id.clone()
            } else if let Some(version_id) = &dependency.version_id {
                client.version(version_id).await?.project_id
            } else {
                return Err(CommandError::new(format!(
                    "Mod {} contains an incomplete required dependency.",
                    version.name
                )));
            };
            if let Some(version_id) = &dependency.version_id {
                record_constraint(
                    &mut discovered_exact_versions,
                    &dependency_project,
                    version_id,
                    &version.name,
                )?;
            }
            queue.push_back(dependency_project);
        }
        resolved.insert(project_id, version);
    }
    Ok((resolved, discovered_exact_versions))
}

fn select_preferred(mut versions: Vec<ProjectVersion>) -> Option<ProjectVersion> {
    if versions.is_empty() {
        return None;
    }
    let preferred = versions
        .iter()
        .position(|version| version.version_type == "release")
        .unwrap_or(0);
    Some(versions.remove(preferred))
}

async fn incompatible_project_error(client: &ModrinthClient, project_id: &str) -> CommandError {
    let project_ids = [project_id.to_string()];
    let title = client
        .projects(&project_ids)
        .await
        .ok()
        .and_then(|projects| projects.into_iter().next())
        .map(|project| project.title)
        .filter(|title| !title.trim().is_empty());

    match title {
        Some(title) => CommandError::new(format!(
            "No compatible version was found for {title} (Modrinth project {project_id})."
        )),
        None => CommandError::new(format!(
            "No compatible version was found for mod {project_id}."
        )),
    }
}

fn validate_exact_version(
    version: &ProjectVersion,
    project_id: &str,
    game: &str,
    loader: &str,
) -> Result<(), CommandError> {
    if version.project_id != project_id {
        return Err(CommandError::new(format!(
            "A dependency for {project_id} points to a version of another project."
        )));
    }
    if (!version.game_versions.is_empty() && !version.game_versions.iter().any(|item| item == game))
        || (!version.loaders.is_empty() && !version.loaders.iter().any(|item| item == loader))
    {
        return Err(CommandError::new(format!(
            "Required mod version {} is not compatible with Minecraft {game} and {loader}.",
            version.name
        )));
    }
    Ok(())
}

fn record_constraint(
    constraints: &mut HashMap<String, String>,
    project_id: &str,
    version_id: &str,
    requested_by: &str,
) -> Result<(), CommandError> {
    if let Some(existing) = constraints.get(project_id) {
        if existing != version_id {
            return Err(CommandError::new(format!(
                "The selected mods require two different versions of {project_id}. One requirement comes from {requested_by}. Remove or update one of the conflicting mods."
            )));
        }
    } else {
        constraints.insert(project_id.to_string(), version_id.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::record_constraint;
    use std::collections::HashMap;

    #[test]
    fn exact_dependency_version_replaces_unconstrained_selection() {
        let mut constraints = HashMap::new();
        record_constraint(&mut constraints, "sodium", "UddlN6L4", "Iris 1.10.7")
            .expect("constraint should be accepted");
        assert_eq!(
            constraints.get("sodium").map(String::as_str),
            Some("UddlN6L4")
        );
    }

    #[test]
    fn conflicting_exact_dependencies_are_reported() {
        let mut constraints = HashMap::from([("sodium".to_string(), "older".to_string())]);
        let error = record_constraint(&mut constraints, "sodium", "newer", "Iris")
            .expect_err("conflict should fail");
        assert!(error.message.contains("two different versions"));
        assert!(error.message.contains("Iris"));
    }
}
