# wpush

**Clone repositories on Windows, push to WSL.**  
Zero-config bridge between two worlds.

[![GitHub stars](https://img.shields.io/github/stars/0xA672/wpush?style=social)](https://github.com/0xA672/wpush/stargazers)
[![Crates.io](https://img.shields.io/crates/v/wpush-git.svg)](https://crates.io/crates/wpush-git)
[![License](https://img.shields.io/crates/l/wpush-git.svg)](https://github.com/0xA672/wpush/blob/main/LICENSE)
[![CI](https://github.com/0xA672/wpush/actions/workflows/ci.yml/badge.svg)](https://github.com/0xA672/wpush/actions/workflows/ci.yml)

---

## Why wpush?

Sometimes you clone a repository on Windows (e.g., from VS Code, File Explorer, or a Windows terminal) but need to work with the code inside WSL.  
Manually copying and converting paths is tedious—`wpush` automates it in one command.

## Prerequisites

- Windows 10/11 with [WSL2](https://learn.microsoft.com/en-us/windows/wsl/install) installed
- [Rust](https://www.rust-lang.org/tools/install) (only if building from source)

## Installation

### Pre-built binaries (recommended)

Download `wpush.exe` from the [Releases](https://github.com/0xA672/wpush/releases) page and place it in a directory that is on your `PATH`.

### Using Cargo

```shell
# From crates.io
cargo install wpush-git

# From source
git clone https://github.com/0xA672/wpush.git
cd wpush
cargo install --path .
```

### Using PowerShell (one‑liner)

Copy and paste the entire script into a **PowerShell** window. Administrator rights are *not* required unless you choose `C:\Windows` as the installation directory.

```powershell
$url = "https://github.com/0xA672/wpush/releases/latest/download/wpush.exe"
$tempFile = "$env:TEMP\wpush.exe"
Invoke-WebRequest -Uri $url -OutFile $tempFile
Unblock-File -Path $tempFile

$destDir = "$env:USERPROFILE\.cargo\bin"
if (-not (Test-Path $destDir)) {
    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
    Write-Host "Created directory: $destDir" -ForegroundColor Cyan
}
Move-Item -Path $tempFile -Destination "$destDir\wpush.exe" -Force

$paths = $env:PATH -split ';'
if ($paths -notcontains $destDir) {
    Write-Warning "  $destDir is NOT in your system PATH."
    Write-Host "   To add it manually, run this command in an elevated PowerShell:" -ForegroundColor Yellow
    Write-Host "   [Environment]::SetEnvironmentVariable('PATH', `$env:PATH + ';$destDir', 'User')" -ForegroundColor Gray
    Write-Host "   Or add '$destDir' via System Properties -> Environment Variables."
} else {
    Write-Host " $destDir is already in your PATH." -ForegroundColor Green
}

Write-Host " wpush installed to $destDir\wpush.exe" -ForegroundColor Green
Write-Host ""
Write-Host " IMPORTANT: Close and reopen your terminal, or refresh environment variables." -ForegroundColor Yellow
Write-Host "   (If you use Chocolatey, you can run 'refreshenv')"
Write-Host "After that, try running: wpush --help"
```

## Usage

```shell
wpush [OPTIONS] <REPO_URL> <DEST_PATH>
```

### Options

| Option                     | Description                                                              |
|----------------------------|--------------------------------------------------------------------------|
| `-d, --distro <DISTRO>`    | WSL distribution name (default: `Ubuntu`)                                 |
| `-b, --branch <BRANCH>`    | Git branch to clone                                                      |
| `-u, --user <USER>`        | WSL username (auto‑detected via `wsl whoami` if omitted)                  |
| `-k, --keep-git`           | Preserve the `.git` directory (keeps full Git history)                    |
| `-x, --exclude <PATTERN>` | Additional directory/file patterns to exclude from copy (repeatable)       |
| `-n, --dry-run`            | Preview actions without actually cloning or copying                       |
| `-h, --help`               | Show help message                                                         |
| `-V, --version`            | Show version information                                                  |

### Destination Path Formats

`wpush` supports two kinds of destination paths inside WSL:

| Format            | Expands to                                           |
|-------------------|------------------------------------------------------|
| `~/myproject`     | `/home/<username>/myproject`                         |
| `~`               | `/home/<username>`                                   |
| `/absolute/path`  | Used as‑is (must exist or be creatable)              |

The username is automatically detected by running `wsl -d <distro> whoami` unless you override it with `--user`.

## Examples

```powershell
# Clone into auto-detected user's home directory
wpush https://github.com/user/repo.git ~/repo

# Preview what would happen (dry run)
wpush https://github.com/user/repo.git ~/repo --dry-run

# Clone a specific branch
wpush https://github.com/user/repo.git ~/repo -b develop

# Clone into a different WSL distro with a specific user
wpush https://github.com/user/repo.git ~/repo --distro Debian --user root

# Use an absolute path inside WSL
wpush https://github.com/user/repo.git /home/cero/projects/repo

# Clone with full Git history preserved
wpush https://github.com/user/repo.git ~/repo --keep-git
```

## Completions
### Run the following commands in a **PowerShell** window:
```powershell
# 1. Ensure your PowerShell profile file exists
if (!(Test-Path -Path $PROFILE)) {
    New-Item -ItemType File -Path $PROFILE -Force
}

# 2. Append the completion script to your profile
wpush completions powershell >> $PROFILE

# 3. Reload your profile to enable completions immediately
. $PROFILE
```

> [!NOTE]
> If you encounter an error like running scripts is disabled on this system, you may need to adjust your PowerShell execution policy first:
> ```powershell
> Set-ExecutionPolicy RemoteSigned -Scope CurrentUser
> ```
After installation, open a new PowerShell window and start typing wpush followed by Tab to see completions for:
* Subcommands (e.g., completions)
* Options (-d, --distro, --keep-git, etc.)
* Dynamic WSL distribution names for the --distro flag

## How It Works

1. **Git clone** – The repository is cloned into a temporary Windows directory, with transfer progress displayed in the terminal.
2. **Path resolution** – The destination path is expanded:
   - `~` and `~/...` are converted to `/home/<username>/...` using the detected or specified WSL user.
   - Absolute paths are kept unchanged.
3. **WSL filesystem copy** – The cloned working tree is copied into `\\wsl$\<distro>\<path>` using `robocopy`.
4. **Cleanup** – The temporary Windows directory is automatically removed when the process finishes.

> [!IMPORTANT]
> By default, the `.git` folder is **excluded** during the copy, so the destination will **not** be a Git repository.  
> Use the `-k` / `--keep-git` flag to preserve the full Git history inside WSL.

---

## Contributing

Contributions, issues, and feature requests are welcome!  
Feel free to check the [issues page](https://github.com/0xA672/wpush/issues).

## License

Licensed under either of  
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)  
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)  

at your option.
