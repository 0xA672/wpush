use clap::Parser;

#[derive(Parser)]
#[command(
    name = "wpush",
    about = "Clone a repo on Windows, push it into WSL",
    long_about = r#"wpush - Windows to WSL Repository Pusher

Clone any Git repository on Windows side, then seamlessly push it into your WSL filesystem.

FEATURES:
  - Git-style progress bars with color output
  - Automatic WSL username detection
  - Support for ~ (tilde) expansion in paths
  - Branch selection
  - Multi-distro support
  - Option to preserve Git history (--keep-git / -k)

DEST PATH FORMATS:
  ~/project          Expands to /home/<username>/project (auto-detects user)
  ~                  Expands to /home/<username>
  /home/user/project Used as-is (absolute path)

EXAMPLES:
  # Clone to auto-detected user's home (without .git)
  wpush https://github.com/user/repo.git ~/project

  # Clone with full Git history
  wpush https://github.com/user/repo.git ~/project --keep-git

  # Clone with specific branch
  wpush https://github.com/user/repo.git ~/project -b develop

  # Clone to different WSL distro
  wpush https://github.com/user/repo.git ~/project --distro Debian

  # Clone with specific user
  wpush https://github.com/user/repo.git ~/project --user root

  # Clone to absolute path
  wpush https://github.com/user/repo.git /home/cero/projects/repo

ENVIRONMENT:
  WSL_DISTRO    Default WSL distro (overridden by --distro)
"#,
    version = env!("CARGO_PKG_VERSION"),
    author
)]
struct Args {
    /// Git repo URL
    repo: String,
    /// Destination path inside WSL
    dest: String,
    /// WSL distro name
    #[arg(short, long, default_value = "Ubuntu")]
    distro: String,
    /// Git branch to clone
    #[arg(short = 'b', long)]
    branch: Option<String>,
    /// WSL username (auto-detected if omitted)
    #[arg(short, long)]
    user: Option<String>,
    /// Keep .git directory (preserve full Git history)
    #[arg(short = 'k', long = "keep-git")]
    keep_git: bool,
    /// Preview the actions without actually cloning or copying
    #[arg(short = 'n', long = "dry-run")]
    dry_run: bool,
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::Args;
    use anyhow::{anyhow, Context, Result};
    use clap::Parser;
    use colored::*;
    use git2::{build::RepoBuilder, FetchOptions, RemoteCallbacks};
    use std::io::{self, StdoutLock, Write};
    use std::process::Command;
    use tempfile::tempdir;

    enum WslPath {
        Absolute(String),
        Tilde(String),
    }

    impl WslPath {
        fn parse(path: &str) -> Self {
            if path.starts_with("~/") {
                WslPath::Tilde(path.to_string())
            } else if path == "~" {
                WslPath::Tilde("~".to_string())
            } else {
                WslPath::Absolute(path.to_string())
            }
        }

        fn resolve(self, distro: &str, user: Option<&str>) -> Result<String> {
            match self {
                WslPath::Absolute(path) => Ok(path),
                WslPath::Tilde(path) => {
                    let username = match user {
                        Some(u) => u.to_string(),
                        None => wsluser(distro)?,
                    };

                    let expanded = if path == "~" {
                        format!("/home/{}", username)
                    } else {
                        format!("/home/{}{}", username, &path[1..])
                    };
                    Ok(expanded)
                }
            }
        }
    }

    fn wsluser(distro: &str) -> Result<String> {
        let output = Command::new("wsl")
            .args(["-d", distro, "whoami"])
            .output()
            .with_context(|| format!("Failed to detect WSL user for distro '{}'", distro))?;

        if !output.status.success() {
            return Err(anyhow!("wsl whoami failed"));
        }

        let user = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if user.is_empty() {
            return Err(anyhow!("WSL user is empty"));
        }

        Ok(user)
    }

    pub fn run() -> Result<()> {
        let stdout = io::stdout();
        let mut locker: StdoutLock = stdout.lock();
        let args: Args = Args::parse();
        let wsl_path = WslPath::parse(&args.dest);
        let resolved_dest = wsl_path.resolve(&args.distro, args.user.as_deref())?;

        writeln!(locker, "{}", format!("Cloning {}...", args.repo).cyan())?;
        if let Some(b) = &args.branch {
            writeln!(locker, "Branch: {}", b.yellow())?;
        }
        writeln!(locker, "Target: {}", resolved_dest.cyan())?;

        if args.dry_run {
            writeln!(locker, "{}", "Dry run completed. No changes made.".green())?;
            return Ok(());
        }

        let tempdir = tempdir()?;
        let clonepath = tempdir.path();

        let mut builder = RepoBuilder::new();
        let mut callbacks = RemoteCallbacks::new();
        callbacks.transfer_progress(|stats| {
            if stats.total_objects() > 0 {
                let pct = (stats.received_objects() * 100) / stats.total_objects();
                eprint!(
                    "\rReceiving objects: {:3}% ({}/{})",
                    pct,
                    stats.received_objects(),
                    stats.total_objects()
                );
            }
            true
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(callbacks);
        builder.fetch_options(fo);

        if let Some(b) = &args.branch {
            builder.branch(b);
        }

        builder.clone(&args.repo, clonepath)?;
        eprintln!("\r{:<50}", "Receiving objects: done.".green());

        let wsldest = format!(
            r"\\wsl$\{}\{}",
            args.distro,
            resolved_dest.trim_start_matches('/')
        );
        std::fs::create_dir_all(&wsldest)
            .with_context(|| format!("failed to create WSL directory {}", wsldest))?;

        let mut robocopy_args = vec![clonepath.to_str().unwrap(), &wsldest, "/E"];
        if !args.keep_git {
            robocopy_args.push("/XD");
            robocopy_args.push(".git");
        }

        let status = Command::new("robocopy").args(&robocopy_args).status()?;

        match status.code() {
            Some(0..=7) => {}
            Some(code) => return Err(anyhow!("robocopy failed with exit code: {}", code)),
            None => return Err(anyhow!("robocopy terminated unexpectedly")),
        }

        writeln!(locker, "{}", "done.".green())?;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    windows_impl::run()
}

#[cfg(not(target_os = "windows"))]
fn main() {
    use colored::*;
    eprintln!(
        "{}",
        "Error: wpush only runs on Windows (requires WSL).".red()
    );
    std::process::exit(1);
}
