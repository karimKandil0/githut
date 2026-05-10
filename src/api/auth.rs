use anyhow::{anyhow, Context, Result};

pub async fn get_token() -> Result<String> {
    let output = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .context("failed to run `gh auth token` — is `gh` installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "gh auth token failed: {}\nRun `gh auth login` first.",
            stderr.trim()
        ));
    }

    let token =
        String::from_utf8(output.stdout).context("gh auth token output is not valid UTF-8")?;
    let token = token.trim().to_string();

    if token.is_empty() {
        return Err(anyhow!(
            "gh auth token returned empty — run `gh auth login` first."
        ));
    }

    Ok(token)
}
