use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};

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
    repo: String,
    dest: String,
    #[arg(short, long, default_value = "Ubuntu")]
    distro: String,
    #[arg(short = 'b', long)]
    branch: Option<String>,
    #[arg(short, long)]
    user: Option<String>,
    #[arg(short = 'k', long = "keep-git")]
    keep_git: bool,
    #[arg(short = 'n', long = "dry-run")]
    dry_run: bool,
    #[arg(long, hide = true, exclusive = true)]
    completions: Option<Shell>,
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::Args;
    use anyhow::{anyhow, bail, Context, Result};
    use colored::*;
    use git2::{build::RepoBuilder, FetchOptions, RemoteCallbacks};
    use std::io::{self, StdoutLock, Write};
    use std::path::Path;
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::tempdir;

    enum WslPath {
        Absolute(String),
        Tilde(String),
    }

    impl WslPath {
        fn parse(path: &str) -> Result<Self> {
            if path.contains("..") {
                bail!("Path contains '..' which is not allowed for security reasons");
            }
            if path.contains('\\') {
                bail!("Backslashes are not allowed in WSL destination paths");
            }

            if path.starts_with("~/") {
                Ok(WslPath::Tilde(path.to_string()))
            } else if path == "~" {
                Ok(WslPath::Tilde("~".to_string()))
            } else {
                if !path.starts_with('/') {
                    bail!("Absolute WSL path must start with '/' (e.g., /home/user/project)");
                }
                Ok(WslPath::Absolute(path.to_string()))
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
            bail!("wsl whoami failed");
        }

        let user = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if user.is_empty() {
            bail!("WSL user is empty");
        }

        if !user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            bail!("Invalid WSL username format");
        }

        Ok(user)
    }

    fn validate_repo_url(url: &str) -> Result<()> {
        if url.trim().is_empty() {
            bail!("Repository URL cannot be empty");
        }
        if !(url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("git@")
            || url.starts_with("file://"))
        {
            bail!("Invalid Git URL format");
        }
        if url.contains(';')
            || url.contains('|')
            || url.contains('&')
            || url.contains('$')
            || url.contains('`')
            || url.contains('\'')
            || url.contains('"')
        {
            bail!("Git URL contains potentially dangerous characters");
        }
        Ok(())
    }

    fn validate_distro(distro: &str) -> Result<()> {
        if distro.trim().is_empty() {
            bail!("Distro name cannot be empty");
        }
        if distro.contains('/')
            || distro.contains('\\')
            || distro.contains(':')
            || distro.contains(';')
            || distro.contains('|')
            || distro.contains('&')
        {
            bail!("Distro name contains invalid characters");
        }
        Ok(())
    }

    fn validate_user(user: &str) -> Result<()> {
        if user.trim().is_empty() {
            bail!("Username cannot be empty");
        }
        if !user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            bail!("Invalid username format");
        }
        Ok(())
    }

    fn print_preview(
        repo: &str,
        branch: Option<&str>,
        dest: &str,
        keep_git: bool,
        mut out: StdoutLock,
    ) -> Result<()> {
        writeln!(out, "{}", format!("Cloning {}...", repo).cyan())?;
        if let Some(b) = branch {
            writeln!(out, "Branch: {}", b.yellow())?;
        }
        writeln!(out, "Target: {}", dest.cyan())?;
        writeln!(
            out,
            "Keep .git: {}",
            if keep_git { "yes".green() } else { "no".red() }
        )?;
        Ok(())
    }

    fn clone_repo(repo_url: &str, branch: Option<&str>, target_dir: &Path) -> Result<()> {
        let mut builder = RepoBuilder::new();
        let mut callbacks = RemoteCallbacks::new();

        let last_reported_pct = AtomicUsize::new(0);
        callbacks.transfer_progress(move |stats| {
            if stats.total_objects() > 0 {
                let pct = (stats.received_objects() * 100) / stats.total_objects();
                let prev = last_reported_pct.load(Ordering::Relaxed);
                if pct >= prev + 10 || pct == 100 || prev == 0 {
                    eprint!(
                        "\rReceiving objects: {:3}% ({}/{})",
                        pct,
                        stats.received_objects(),
                        stats.total_objects()
                    );
                    last_reported_pct.store(pct, Ordering::Relaxed);
                }
            }
            true
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(callbacks);
        builder.fetch_options(fo);

        if let Some(b) = branch {
            builder.branch(b);
        }

        builder
            .clone(repo_url, target_dir)
            .with_context(|| format!("Failed to clone repository: {}", repo_url))?;

        eprintln!("\r{:<50}", "Receiving objects: done.".green());
        Ok(())
    }

    fn copy_to_wsl(source: &Path, wsl_dest: &str, keep_git: bool) -> Result<()> {
        std::fs::create_dir_all(wsl_dest)
            .with_context(|| format!("failed to create WSL directory {}", wsl_dest))?;

        let mut robocopy_args = vec![
            source
                .to_str()
                .ok_or_else(|| anyhow!("Invalid source path encoding"))?,
            wsl_dest,
            "/E",
        ];

        if !keep_git {
            robocopy_args.push("/XD");
            robocopy_args.push(".git");
        }

        let status = Command::new("robocopy").args(&robocopy_args).status()?;

        match status.code() {
            Some(0..=7) => Ok(()),
            Some(code) => Err(anyhow!("robocopy failed with exit code: {}", code)),
            None => Err(anyhow!("robocopy terminated unexpectedly")),
        }
    }

    pub fn run() -> Result<()> {
        let stdout = io::stdout();
        let locker = stdout.lock();

        let args = Args::parse();

        if let Some(shell) = args.completions {
            let mut cmd = Args::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut io::stdout());
            return Ok(());
        }

        validate_repo_url(&args.repo)?;
        validate_distro(&args.distro)?;
        if let Some(ref u) = args.user {
            validate_user(u)?;
        }

        let wsl_path = WslPath::parse(&args.dest)?;
        let resolved_dest = wsl_path.resolve(&args.distro, args.user.as_deref())?;

        print_preview(
            &args.repo,
            args.branch.as_deref(),
            &resolved_dest,
            args.keep_git,
            locker,
        )?;

        if args.dry_run {
            println!("{}", "Dry run completed. No changes made.".green());
            return Ok(());
        }

        let tempdir = tempdir()?;
        let clonepath = tempdir.path();

        clone_repo(&args.repo, args.branch.as_deref(), clonepath)?;

        let wsl_internal_path = resolved_dest.trim_start_matches('/').replace('/', "\\");
        let wsldest = format!(r"\\wsl$\{}\{}", args.distro, wsl_internal_path);

        copy_to_wsl(clonepath, &wsldest, args.keep_git)?;

        println!("{}", "done.".green());
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
