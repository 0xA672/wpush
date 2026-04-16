# wpush
Clone repos on Windows, push to WSL. Zero-config bridge between worlds.

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
```
## How It Works

1. **Git clone** – The repository is cloned into a temporary Windows directory, with transfer progress shown in the terminal.
2. **Path resolution** – The destination path is expanded:
   - `~` and `~/...` are converted to `/home/<username>/...` using the detected or specified WSL user.
   - Absolute paths are kept unchanged.
3. **WSL filesystem copy** – The cloned working tree is copied into `\\wsl$\<distro>\<path>` using `robocopy`.
4. **Cleanup** – The temporary Windows directory is automatically deleted when the process finishes.

> [!IMPORTANT]
> The `.git` directory is **intentionally excluded** during the copy.
> This means the destination folder **will not be a Git repository** – it contains only the latest source code, not the version history.  
> If you need a full clone inside WSL, use `git clone` directly from your WSL terminal.
