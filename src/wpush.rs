use std::io::{self, StdoutLock, Write};
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::process::Command;
use tempfile::tempdir;
use git2::{build::RepoBuilder, FetchOptions, RemoteCallbacks};
use colored::*;

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
    
    let user = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();
    
    if user.is_empty() {
        return Err(anyhow!("WSL user is empty"));
    }
    
    Ok(user)
}

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

DEST PATH FORMATS:
  ~/project          Expands to /home/<username>/project (auto-detects user)
  ~                  Expands to /home/<username>
  /home/user/project Used as-is (absolute path)

EXAMPLES:
  # Clone to auto-detected user's home
  wpush https://github.com/user/repo.git ~/project

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
    version,
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
}



fn main() -> Result<()> {
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
    
    let tempdir = tempdir()?;
    let clonepath = tempdir.path();
    let mut bulider: RepoBuilder = RepoBuilder::new();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.transfer_progress(|stats| {
        if stats.total_objects() > 0 {
            let pct = (stats.received_objects() * 100) / stats.total_objects();
            eprint!("\rReceiving objects: {:3}% ({}/{})",
                    pct, stats.received_objects(), stats.total_objects());
        }
        true
    });
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks);
    bulider.fetch_options(fo);
    if let Some(b) = &args.branch{
      bulider.branch(b);
    }
    bulider.clone(&args.repo,clonepath)?;
    eprintln!(", done.");
    
    let wsldest = format!(r"\\wsl$\{}\{}", args.distro, resolved_dest.trim_start_matches('/'));
    std::fs::create_dir_all(&wsldest)
        .with_context(|| format!("failed to create WSL directory {}", wsldest))?;
    let status = Command::new("robocopy")
        .args([clonepath.to_str().unwrap(),&wsldest,"/E","/XD",".git",]).status()?;
    // robocopy exit codes: 0-7 = success, 8+ = error
    if status.code().unwrap_or(8) >= 8 {
        return Err(anyhow!("robocopy failed, exit code: {:?}", status.code()));
    }
    writeln!(locker, "{}", "done.".green())?;
    Ok(())
}
