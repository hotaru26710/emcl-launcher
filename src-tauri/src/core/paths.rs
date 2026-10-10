use std::env;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

#[derive(Clone, Debug)]
pub struct LauncherPaths {
    root: PathBuf,
}

impl LauncherPaths {
    pub fn discover() -> Result<Self> {
        let root = env::var_os("EMCL_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".emcl")))
            .context("unable to resolve the user home directory")?;

        Ok(Self { root })
    }

    pub fn from_root(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn shared_dir(&self) -> PathBuf {
        self.root.join("shared")
    }

    pub fn versions_dir(&self) -> PathBuf {
        self.shared_dir().join("versions")
    }

    pub fn version_dir(&self, version_id: &str) -> PathBuf {
        self.versions_dir().join(version_id)
    }

    pub fn libraries_dir(&self) -> PathBuf {
        self.shared_dir().join("libraries")
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.shared_dir().join("assets")
    }

    pub fn instances_dir(&self) -> PathBuf {
        self.root.join("instances")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn database_file(&self) -> PathBuf {
        self.root.join("launcher.db")
    }

    pub fn ensure(&self) -> Result<()> {
        for path in [
            self.root.clone(),
            self.shared_dir(),
            self.versions_dir(),
            self.libraries_dir(),
            self.assets_dir(),
            self.instances_dir(),
            self.logs_dir(),
        ] {
            std::fs::create_dir_all(&path)
                .with_context(|| format!("failed to create {}", path.display()))?;
        }
        Ok(())
    }

    pub fn instance_dir(&self, instance_id: &str) -> Result<PathBuf> {
        if !is_safe_segment(instance_id) {
            bail!("invalid instance id");
        }

        Ok(self.instances_dir().join(instance_id))
    }
}

pub fn is_safe_segment(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '+')
        })
}
