use crate::error::CommandError;
use serde::{Deserialize, Serialize};

const API: &str = "https://api.modrinth.com/v2";

#[derive(Clone)]
pub struct ModrinthClient(reqwest::Client);

impl ModrinthClient {
    pub fn new() -> Result<Self, CommandError> {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "NinjaLauncher/",
                env!("CARGO_PKG_VERSION"),
                " (contact: local desktop app)"
            ))
            .https_only(true)
            .build()?;
        Ok(Self(client))
    }

    pub async fn search(
        &self,
        query: &str,
        game_version: &str,
        loader: &str,
        offset: u32,
    ) -> Result<SearchResponse, CommandError> {
        self.search_projects(query, Some(game_version), loader, "mod", offset)
            .await
    }

    pub async fn search_modpacks(
        &self,
        query: &str,
        loader: &str,
        offset: u32,
    ) -> Result<SearchResponse, CommandError> {
        self.search_projects(query, None, loader, "modpack", offset)
            .await
    }

    async fn search_projects(
        &self,
        query: &str,
        game_version: Option<&str>,
        loader: &str,
        project_type: &str,
        offset: u32,
    ) -> Result<SearchResponse, CommandError> {
        let mut facets = vec![
            vec![format!("project_type:{project_type}")],
            vec![format!("categories:{loader}")],
        ];
        if let Some(game_version) = game_version {
            facets.push(vec![format!("versions:{game_version}")]);
        }
        let facets = serde_json::to_string(&facets)?;
        let offset = offset.to_string();
        let response = self
            .0
            .get(format!("{API}/search"))
            .query(&[
                ("query", query),
                ("facets", &facets),
                ("limit", "20"),
                ("offset", &offset),
                ("index", "relevance"),
            ])
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.0
    }

    pub async fn versions(
        &self,
        project_id: &str,
        game_version: &str,
        loader: &str,
    ) -> Result<Vec<ProjectVersion>, CommandError> {
        let game_versions = serde_json::to_string(&[game_version])?;
        let loaders = serde_json::to_string(&[loader])?;
        let response = self
            .0
            .get(format!("{API}/project/{project_id}/version"))
            .query(&[("game_versions", game_versions), ("loaders", loaders)])
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    }

    pub async fn versions_for_loader(
        &self,
        project_id: &str,
        loader: &str,
    ) -> Result<Vec<ProjectVersion>, CommandError> {
        let loaders = serde_json::to_string(&[loader])?;
        let response = self
            .0
            .get(format!("{API}/project/{project_id}/version"))
            .query(&[("loaders", loaders)])
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    }

    pub async fn projects(
        &self,
        project_ids: &[String],
    ) -> Result<Vec<ProjectDetails>, CommandError> {
        if project_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = serde_json::to_string(project_ids)?;
        let response = self
            .0
            .get(format!("{API}/projects"))
            .query(&[("ids", ids)])
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json().await?)
    }

    pub async fn version(&self, version_id: &str) -> Result<ProjectVersion, CommandError> {
        Ok(self
            .0
            .get(format!("{API}/version/{version_id}"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub total_hits: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub categories: Vec<String>,
    pub versions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct ProjectDetails {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub icon_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProjectVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_type: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectVersionSummary {
    pub id: String,
    pub name: String,
    pub version_type: String,
    pub game_versions: Vec<String>,
}

impl From<&ProjectVersion> for ProjectVersionSummary {
    fn from(version: &ProjectVersion) -> Self {
        Self {
            id: version.id.clone(),
            name: version.name.clone(),
            version_type: version.version_type.clone(),
            game_versions: version.game_versions.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct VersionFile {
    pub hashes: FileHashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FileHashes {
    pub sha1: String,
    pub sha512: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Dependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub dependency_type: String,
}

#[cfg(test)]
mod tests {
    use super::SearchResponse;

    #[test]
    fn converts_modrinth_snake_case_fields_for_the_frontend() {
        let response: SearchResponse = serde_json::from_str(
            r#"{
                "hits": [{
                    "project_id": "AANobbMI",
                    "slug": "sodium",
                    "title": "Sodium",
                    "description": "Rendering engine",
                    "author": "jellysquid3",
                    "downloads": 123,
                    "icon_url": "https://cdn.modrinth.com/icon.webp",
                    "categories": ["fabric", "optimization"],
                    "versions": ["26.2"]
                }],
                "total_hits": 1
            }"#,
        )
        .expect("Modrinth search response should deserialize");

        assert_eq!(response.hits[0].project_id, "AANobbMI");
        let frontend = serde_json::to_value(response).expect("response should serialize");
        assert_eq!(frontend["hits"][0]["projectId"], "AANobbMI");
        assert_eq!(
            frontend["hits"][0]["iconUrl"],
            "https://cdn.modrinth.com/icon.webp"
        );
    }
}
