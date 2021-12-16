use anyhow::Result;
use command_run::Command;
use fs_err as fs;
use std::env;
use std::path::Path;

pub fn publish() -> Result<()> {
    // Setup notes:
    // - Create a personal access token with the `public_repo` scope
    // - repo settings > Environments > github-pages > Add secret
    //   - Name: TOKEN

    let branch = "gh-pages";
    let url = "https://github.com/nicholasbishop/nbishop-net.git";
    let repo_path = "repo";

    // Delete the repo if it already exists.
    if Path::new(repo_path).exists() {
        fs::remove_dir_all(repo_path)?;
    }

    // Create an empty repo.
    Command::with_args("git", &["init", repo_path]).run()?;

    let set_config = |key, val| {
        Command::with_args("git", &["-C", repo_path, "config", key, val]).run()
    };

    // Configure the credential helper. The credential helper helps
    // avoid showing the private token.
    let helper_path = env::current_dir()?.join("src/credential_helper.sh");
    set_config("credential.helper", helper_path.to_str().unwrap())?;

    // Set identify, required for commit.
    set_config("user.email", "publisher@nbishop.net")?;
    set_config("user.name", "Automatic Publisher")?;

    // Add the remote.
    Command::with_args(
        "git",
        &["-C", repo_path, "remote", "add", "origin", url],
    )
    .run()?;

    // Fetch the branch.
    Command::with_args("git", &["-C", repo_path, "fetch", "origin", branch])
        .run()?;

    // Check out the branch.
    Command::with_args("git", &["-C", repo_path, "checkout", branch]).run()?;

    // Delete all the files currently present. Allow this to fail in
    // case there's nothing there yet.
    Command::with_args("git", &["-C", repo_path, "rm", "*"])
        .disable_check()
        .run()?;

    // Copy all the output files into the repo.
    // TODO: deduplicate path.
    let output_dir = "output";
    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        fs::copy(entry.path(), Path::new(repo_path).join(entry.file_name()))?;
    }

    // Add all the files.
    Command::with_args("git", &["-C", repo_path, "add", "*"]).run()?;

    // Commit.
    Command::with_args(
        "git",
        &["-C", repo_path, "commit", "-m", "Automatic publish"],
    )
    .run()?;

    // Push.
    Command::with_args("git", &["-C", repo_path, "push"]).run()?;

    Ok(())
}
