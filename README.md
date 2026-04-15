# wpush
Clone repos on Windows, push to WSL. Zero-config bridge between worlds.

## Prerequisites
- Windows 10/11 with [WSL2](https://learn.microsoft.com/en-us/windows/wsl/install) installed
- [Rust](https://www.rust-lang.org/tools/install) (if building from source)
  
`
## Installation

### Pre-built binaries (recommended)
Download the latest `wpush.exe` from the [Releases](https://github.com/0xA672/wpush/releases) page and place it in a directory that's in your `PATH`.

### Using Cargo (from source)
```shell
git clone https://github.com/0xA672/wpush.git
cd wpush
cargo install --path .
```
