use anyhow::{Context, Result};

pub fn clone_repo(url: &str, path: &str) -> Result<()> {
    git2::Repository::clone(url, path)
        .with_context(|| format!("failed to clone {} into {}", url, path))?;
    Ok(())
}
