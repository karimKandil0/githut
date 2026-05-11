use anyhow::{Context, Result};
use git2::{FetchOptions, Repository};
use std::path::Path;

pub fn clone_repo(url: &str, path: &str) -> Result<()> {
    Repository::clone(url, path)
        .with_context(|| format!("failed to clone {} into {}", url, path))?;
    Ok(())
}

/// Sparse clone: init repo, enable sparse checkout, fetch, checkout only `dirs`.
/// `dirs` is a slice of path patterns e.g. ["src", "docs"].
pub fn sparse_clone(url: &str, path: &str, branch: &str, dirs: &[&str]) -> Result<()> {
    let repo =
        Repository::init(path).with_context(|| format!("failed to init repo at {}", path))?;

    // Add remote
    let mut remote = repo.remote("origin", url).context("failed to add remote")?;

    // Enable sparse checkout in config
    let mut config = repo.config().context("failed to get repo config")?;
    config
        .set_bool("core.sparseCheckout", true)
        .context("failed to set core.sparseCheckout")?;

    // Write sparse-checkout patterns
    let sparse_file = Path::new(path).join(".git/info/sparse-checkout");
    let patterns = if dirs.is_empty() {
        "/*\n".to_string()
    } else {
        dirs.iter()
            .map(|d| format!("{}\n", d.trim_matches('/')))
            .collect::<String>()
    };
    std::fs::write(&sparse_file, &patterns).context("failed to write sparse-checkout file")?;

    // Fetch
    let refspec = format!("refs/heads/{}:refs/remotes/origin/{}", branch, branch);
    let mut fetch_opts = FetchOptions::new();
    remote
        .fetch(&[&refspec], Some(&mut fetch_opts), None)
        .context("fetch failed")?;

    // Find the fetched commit and set HEAD + checkout
    let fetch_head = repo
        .find_reference(&format!("refs/remotes/origin/{}", branch))
        .context("failed to find fetched branch")?;
    let commit = repo
        .reference_to_annotated_commit(&fetch_head)
        .context("failed to resolve commit")?;
    let object = repo
        .find_object(commit.id(), None)
        .context("failed to find object")?;

    repo.set_head(&format!("refs/heads/{}", branch))
        .context("failed to set HEAD")?;
    repo.checkout_tree(&object, None)
        .context("checkout failed")?;

    Ok(())
}
