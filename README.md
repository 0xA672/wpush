# wpush
Clone repos on Windows, push to WSL. Zero-config bridge between worlds.
[![GitHub stars](https://img.shields.io/github/stars/0xA672/wpush?style=social)](https://github.com/0xA672/wpush/stargazers)

[![Crates.io](https://img.shields.io/crates/v/wpush-git.svg)](https://crates.io/crates/wpush-git)
[![License](https://img.shields.io/crates/l/wpush-git.svg)](https://github.com/0xA672/wpush/blob/main/LICENSE)


## Why wpush?

Sometimes you clone a repo on Windows (e.g., from VS Code, File Explorer, or a Windows terminal) but want to work with the code inside WSL. Manual copying and path conversion is annoying.

## Prerequisites
- Windows 10/11 with [WSL2](https://learn.microsoft.com/en-us/windows/wsl/install) installed
- [Rust](https://www.rust-lang.org/tools/install) (if building from source)
  

## Installation

### Pre-built binaries (recommended)

Download `wpush.exe` from [Releases](https://github.com/0xA672/wpush/releases) and place it in a directory in your `PATH`.

### Using Cargo

```shell
# From crates.io
cargo install wpush-git
```
```shell
# From source
git clone https://github.com/0xA672/wpush.git
cd wpush
cargo install --path .
```

### Using PowerShell 

Copy and paste the entire script into a **PowerShell** window (Administrator rights are *not* required unless you choose `C:\Windows` as the install directory).

```powershell
# Download the latest wpush.exe
$url = "https://github.com/0xA672/wpush/releases/latest/download/wpush.exe"
$tempFile = "$env:TEMP\wpush.exe"
Invoke-WebRequest -Uri $url -OutFile $tempFile

# Remove "Mark of the Web" to prevent SmartScreen blocking
Unblock-File -Path $tempFile

# Choose installation directory (must be in your PATH, or you will be warned)
$destDir = "$env:USERPROFILE\.cargo\bin"
if (-not (Test-Path $destDir)) {
    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
    Write-Host "Created directory: $destDir" -ForegroundColor Cyan
}

# Move the executable
Move-Item -Path $tempFile -Destination "$destDir\wpush.exe" -Force

# Verify the destination is in PATH
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
Write-Host "   (If you have Chocolatey, you can run 'refreshenv')"
Write-Host "After that, try running: wpush --help"
### Usage
```shell
wpush [OPTIONS] <REPO_URL> <DEST_PATH>
```
## Options

| Option | Description |
|--------|-------------|
| `-d, --distro <DISTRO>` | WSL distribution name (default: `Ubuntu`) |
| `-b, --branch <BRANCH>` | Git branch to clone |
| `-u, --user <USER>` | WSL username (auto-detected via `wsl whoami` if omitted) |
| `-h, --help` | Show help message |
| `-V, --version` | Show version information |
| `-k, --keep-git` | Preserve the `.git` directory during copy (keeps full Git history) |

## Destination Path Formats

`wpush` supports two types of destination paths inside WSL:

| Format | Expands to |
|--------|------------|
| `~/myproject` | `/home/<username>/myproject` |
| `~` | `/home/<username>` |
| `/absolute/path` | Used as-is (must exist or be creatable) |

The username is automatically detected by running `wsl -d <distro> whoami` unless overridden with `--user`.

## Examples

```powershell
# Clone into auto-detected user's home directory
wpush https://github.com/user/repo.git ~/repo

# Clone a specific branch
wpush https://github.com/user/repo.git ~/repo -b develop

# Clone into a different WSL distro with a specific user
wpush https://github.com/user/repo.git ~/repo --distro Debian --user root

# Use an absolute path inside WSL
wpush https://github.com/user/repo.git /home/cero/projects/repo

# Clone with full Git history preserved
wpush https://github.com/user/repo.git ~/repo --keep-git
```
## How It Works

1. **Git clone** – The repository is cloned into a temporary Windows directory, with transfer progress shown in the terminal.
2. **Path resolution** – The destination path is expanded:
   - `~` and `~/...` are converted to `/home/<username>/...` using the detected or specified WSL user.
   - Absolute paths are kept unchanged.
3. **WSL filesystem copy** – The cloned working tree is copied into `\\wsl$\<distro>\<path>` using `robocopy`.
4. **Cleanup** – The temporary Windows directory is automatically deleted when the process finishes.

> [!IMPORTANT]
> By default, the `.git` directory is **excluded** during the copy, so the destination folder will **not be a Git repository**.
> Use the `-k` / `--keep-git` flag to preserve the full Git history inside WSL.
