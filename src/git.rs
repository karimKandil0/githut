use anyhow::{anyhow, Context, Result};
use git2::Repository;

pub fn clone_repo(url: &str, path: &str) -> Result<()> {
    Repository::clone(url, path)
        .with_context(|| format!("failed to clone {} into {}", url, path))?;
    Ok(())
}

/// Sparse clone using git CLI — reliable sparse checkout support.
/// `dirs` is a slice of path patterns e.g. ["src", "docs"].
pub fn sparse_clone(url: &str, path: &str, _branch: &str, dirs: &[&str]) -> Result<()> {
    // git clone --no-checkout --filter=blob:none --sparse <url> <path>
    let status = std::process::Command::new("git")
        .args([
            "clone",
            "--no-checkout",
            "--filter=blob:none",
            "--sparse",
            url,
            path,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("failed to run git clone")?;

    if !status.success() {
        return Err(anyhow!("git clone failed"));
    }

    if !dirs.is_empty() {
        // git sparse-checkout set <dirs...>
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("sparse-checkout")
            .arg("set")
            .args(dirs)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .context("failed to run git sparse-checkout set")?;

        if !status.success() {
            return Err(anyhow!("git sparse-checkout set failed"));
        }
    }

    // git checkout
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("checkout")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context("failed to run git checkout")?;

    if !status.success() {
        return Err(anyhow!("git checkout failed"));
    }

    Ok(())
}
