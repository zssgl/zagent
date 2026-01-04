use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WorkflowSpecFile {
    workflow_id: String,
    version: String,
    input_schema: String,
    output_schema: String,
    thresholds: Option<String>,
    rules: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowSpec {
    pub workflow_id: String,
    pub version: String,
    pub input_schema: String,
    pub output_schema: String,
    pub thresholds: Option<String>,
    pub rules: Option<String>,
    pub status: Option<String>,
    pub base_dir: PathBuf,
}

impl WorkflowSpec {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| format!("read workflow spec failed: {}", err))?;
        let spec: WorkflowSpecFile =
            serde_yaml::from_str(&content).map_err(|err| format!("invalid workflow spec: {}", err))?;
        let base_dir = path
            .parent()
            .ok_or_else(|| "workflow spec missing parent dir".to_string())?
            .to_path_buf();
        Ok(Self {
            workflow_id: spec.workflow_id,
            version: spec.version,
            input_schema: spec.input_schema,
            output_schema: spec.output_schema,
            thresholds: spec.thresholds,
            rules: spec.rules,
            status: spec.status,
            base_dir,
        })
    }

    pub fn input_schema_path(&self) -> PathBuf {
        self.base_dir.join(&self.input_schema)
    }

    pub fn output_schema_path(&self) -> PathBuf {
        self.base_dir.join(&self.output_schema)
    }

    pub fn thresholds_path(&self) -> Option<PathBuf> {
        self.thresholds.as_ref().map(|p| self.base_dir.join(p))
    }

    pub fn rules_path(&self) -> Option<PathBuf> {
        self.rules.as_ref().map(|p| self.base_dir.join(p))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct SemVer(pub u32, pub u32, pub u32);

impl SemVer {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.strip_prefix('v').unwrap_or(text);
        let mut parts = text.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        Some(Self(major, minor, patch))
    }
}

pub fn discover_latest_active_version(workflow_root: &Path) -> Result<PathBuf, String> {
    let dir = std::fs::read_dir(workflow_root)
        .map_err(|err| format!("read workflow root failed: {}", err))?;
    let mut best: Option<(SemVer, PathBuf)> = None;
    for entry in dir {
        let entry = entry.map_err(|err| format!("read workflow root entry failed: {}", err))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let file_name = entry.file_name();
        let folder = file_name.to_string_lossy();
        if !folder.starts_with('v') {
            continue;
        }
        let Some(version) = SemVer::parse(&folder) else {
            continue;
        };
        let spec_path = path.join("workflow.yml");
        if !spec_path.exists() {
            continue;
        }
        let spec = WorkflowSpec::load(&spec_path)?;
        if spec
            .status
            .as_deref()
            .is_some_and(|status| status.eq_ignore_ascii_case("active"))
        {
            if best.as_ref().map(|(v, _)| version > *v).unwrap_or(true) {
                best = Some((version, spec_path));
            }
        }
    }
    best.map(|(_, path)| path)
        .ok_or_else(|| format!("no active workflow.yml found under {}", workflow_root.display()))
}

