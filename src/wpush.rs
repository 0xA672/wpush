use clap::Parser;
use clap_complete::Shell;
use indoc::indoc;

#[derive(Parser)]
#[command(
    name = "wpush",
    about = "Clone a repo on Windows, push it into WSL",
    long_about = indoc! {r#"
        wpush - Windows to WSL Repository Pusher

        Clone any Git repository on Windows, then seamlessly push it into your WSL filesystem.

        USAGE:
          wpush [OPTIONS] <REPO_URL> <DEST_PATH>

        ARGUMENTS:
          <REPO_URL>   Git repository URL (https://, git@, or file://)
          <DEST_PATH>  Destination path inside WSL (supports ~ expansion)

        OPTIONS:
          -d, --distro <DISTRO>   WSL distribution name [default: Ubuntu]
          -b, --branch <BRANCH>   Specific branch to clone
          -u, --user <USER>       WSL username (overrides auto-detection)
          -k, --keep-git          Preserve .git directory (history)
          -n, --dry-run           Preview operation without executing
          -h, --help              Print help (see more with '--help')
          -V, --version           Print version

        FEATURES:
          • Git-style progress bars with color output
          • Automatic WSL username detection (via `wsl -d <distro> whoami`)
          • ~ (tilde) expansion to /home/<username>
          • Branch selection, multi-distro support
          • Option to keep full Git history (--keep-git / -k)

        DESTINATION PATH FORMATS:
          ~/project          Expands to /home/<username>/project (auto-detects user)
          ~                  Expands to /home/<username>
          /home/user/project Used as-is (absolute path)

        EXAMPLES:
          # Clone to auto-detected user's home (without .git)
          wpush https://github.com/user/repo.git ~/project

          # Clone with full Git history preserved
          wpush https://github.com/user/repo.git ~/project --keep-git

          # Clone a specific branch
          wpush https://github.com/user/repo.git ~/project -b develop

          # Use a different WSL distro
          wpush https://github.com/user/repo.git ~/project --distro Debian

          # Specify an explicit WSL user
          wpush https://github.com/user/repo.git ~/project --user root

          # Clone to an absolute path (no expansion)
          wpush https://github.com/user/repo.git /home/cero/projects/repo

        ENVIRONMENT VARIABLES:
          WSL_DISTRO    Default WSL distro (overridden by --distro)

        NOTE:
          This tool clones the repository to a temporary location on Windows first,
          then copies the contents into WSL using `robocopy`. The .git directory is
          excluded by default unless --keep-git is specified.
    "#},
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
    use clap::{CommandFactory, Parser};
    use clap_complete::generate;
    use console::style;
    use indicatif::{ProgressBar, ProgressStyle};
    use std::io;
    use std::path::Path;
    use std::process::Command;
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

    // ── Console output helpers ──────────────────

    fn info(msg: &str) {
        println!("{} {}", style("→").cyan(), msg);
    }

    fn success(msg: &str) {
        println!("{} {}", style("✔").green(), msg);
    }

    #[allow(dead_code)]
    fn warn(msg: &str) {
        println!("{} {}", style("⚠").yellow(), msg);
    }

    #[allow(dead_code)]
    fn error(msg: &str) {
        eprintln!("{} {}", style("✖").red(), msg);
    }

    fn create_progress_bar(total: u64) -> ProgressBar {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb
    }

    // ── Preview ─────────────────────────────────

    fn print_preview(repo: &str, branch: Option<&str>, dest: &str, keep_git: bool) -> Result<()> {
        println!();
        info(&format!("Repository: {}", style(repo).white().bold()));
        if let Some(b) = branch {
            info(&format!("Branch:     {}", style(b).yellow()));
        }
        info(&format!("Target:     {}", style(dest).cyan()));
        let keep_str = if keep_git {
            style("yes (preserve .git)").green().bold()
        } else {
            style("no (without .git)").red()
        };
        info(&format!("Keep .git:  {}", keep_str));
        println!();
        Ok(())
    }

    // ── Clone (with progress bar) ───────────────

    fn clone_repo(repo_url: &str, branch: Option<&str>, target_dir: &Path) -> Result<()> {
        let mut builder = git2::build::RepoBuilder::new();
        let mut callbacks = git2::RemoteCallbacks::new();

        let pb = create_progress_bar(100);
        pb.set_message("cloning...");

        // Clone the progress bar so we can move a handle into the closure
        let pb_clone = pb.clone();
        callbacks.transfer_progress(move |stats| {
            if stats.total_objects() > 0 {
                let total = stats.total_objects() as u64;
                let received = stats.received_objects() as u64;
                pb_clone.set_length(total);
                pb_clone.set_position(received);
            }
            true
        });

        let mut fo = git2::FetchOptions::new();
        fo.remote_callbacks(callbacks);
        builder.fetch_options(fo);

        if let Some(b) = branch {
            builder.branch(b);
        }

        builder
            .clone(repo_url, target_dir)
            .with_context(|| format!("Failed to clone {}", repo_url))?;

        pb.finish_and_clear();
        success("Repository cloned successfully");
        Ok(())
    }

    // ── Copy to WSL ────────────────────────────

    fn copy_to_wsl(source: &Path, wsl_dest: &str, keep_git: bool) -> Result<()> {
        info("Copying files to WSL...");
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

        let status = Command::new("robocopy")
            .args(&robocopy_args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;

        match status.code() {
            Some(0..=7) => {
                success("Files copied to WSL successfully");
                Ok(())
            }
            Some(code) => Err(anyhow!("robocopy failed with exit code: {}", code)),
            None => Err(anyhow!("robocopy terminated unexpectedly")),
        }
    }

    // ── Main entry point ────────────────────────

    pub fn run() -> Result<()> {
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
        )?;

        if args.dry_run {
            success("Dry run completed. No changes made.");
            return Ok(());
        }

        let tempdir = tempdir()?;
        let clonepath = tempdir.path();

        clone_repo(&args.repo, args.branch.as_deref(), clonepath)?;

        let wsl_internal_path = resolved_dest.trim_start_matches('/').replace('/', "\\");
        let wsldest = format!(r"\\wsl$\{}\{}", args.distro, wsl_internal_path);
        copy_to_wsl(clonepath, &wsldest, args.keep_git)?;

        success("All done! Repository pushed to WSL.");
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    windows_impl::run()
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!(
        "{}",
        console::style("Error: wpush only runs on Windows (requires WSL).").red()
    );
    std::process::exit(1);
}
