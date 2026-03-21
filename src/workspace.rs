use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

pub struct PreparedWorkspace {
    pub target_dir: PathBuf,
    pub existed: bool,
}

pub fn prepare(requested_path: &Path, force: bool, dry_run: bool) -> Result<PreparedWorkspace> {
    let target_dir = absolute_path(requested_path)?;

    let existed = target_dir.exists();
    if existed {
        if !target_dir.is_dir() {
            bail!("workspace path {} is not a directory", target_dir.display());
        }

        let mut entries = fs::read_dir(&target_dir)
            .with_context(|| format!("failed to read {}", target_dir.display()))?;
        if entries.next().is_some() && !force {
            bail!(
                "workspace directory {} is not empty; use --force to initialize anyway",
                target_dir.display()
            );
        }
    } else if !dry_run {
        fs::create_dir_all(&target_dir)
            .with_context(|| format!("failed to create {}", target_dir.display()))?;
    }

    Ok(PreparedWorkspace {
        target_dir,
        existed,
    })
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path)
            .with_context(|| format!("failed to resolve {}", path.display()));
    }

    let current_dir = std::env::current_dir().context("failed to read current directory")?;
    Ok(current_dir.join(path))
}
