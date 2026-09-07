<div align="center">

  ![Logo](./assets/fsel.png)

*(fast select)*

  [![License](https://img.shields.io/crates/l/fsel?style=flat-square)](https://github.com/Mjoyufull/fsel/blob/main/LICENSE)
  ![written in Rust](https://img.shields.io/badge/language-rust-red.svg?style=flat-square)

  Fast TUI app launcher and fuzzy finder for GNU/Linux and \*BSD

  <img width="860" height="1019" alt="Screenshot_20251006-032156" src="https://github.com/user-attachments/assets/777bd0a4-eb52-4014-837b-d361ab57cfff" />



</div>

## Table of Contents

- [Requirements](#requirements)
- [Quickstart](#quickstart)
- [Documentation](#documentation)
- [Install](#install)
- [Usage](#usage)
- [Configuration](#configuration)
  - [Environment variable overrides](#environment-variable-overrides)
- [Contributing](#contributing)
- [Troubleshooting](#troubleshooting)
- [License](#license)

**Start Here:** [Detailed Usage Guide](./USAGE.md)

## Requirements

**Build Requirements:**
- Rust 1.94+ **stable** (NOT nightly)
  - Install: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - Verify: `rustc --version` (should show stable, not nightly)
  - If using nightly: `rustup default stable`
- Cargo (comes with Rust)

**Runtime Requirements:**
- GNU/Linux or *BSD
- Terminal emulator

**Optional:**
- [`cclip`](https://github.com/heather7283/cclip) - for clipboard history mode
- Kitty, Sixel-, or Halfblocks-capable terminal - for native inline image previews in cclip mode (see [ratatui-image](https://github.com/benjajaja/ratatui-image))

**Note:** Image previews in cclip mode use built-in [ratatui-image](https://github.com/benjajaja/ratatui-image) (no external viewer). Versions before 3.1.0 required `chafa` for image previews; 3.1.0 and later do not.

## Quickstart

Get up and running in 30 seconds:

```sh
# Install with Nix (recommended)
nix run github:Mjoyufull/fsel

# Or install from Cargo
cargo install fsel@3.7.0-kiwicrab

# Launch fsel
fsel

# Use as dmenu replacement
echo -e "Option 1\nOption 2\nOption 3" | fsel --dmenu

# Browse clipboard history (requires cclip)
fsel --cclip
```

That's it. Type to search, arrow keys to navigate, Enter to launch.

Need install variants, launch methods, or mode-specific examples? See [USAGE.md](./USAGE.md).


## Install

#### Option 1: Nix Flake (Recommended)

* Build and run with Nix flakes:
    ```sh
    $ nix run github:Mjoyufull/fsel
    ```

* Add to your profile:
    ```sh
    $ nix profile add github:Mjoyufull/fsel
    ```

* Add to your `flake.nix` inputs:
    ```nix
    {
      inputs.fsel.url = "github:Mjoyufull/fsel";
      # ... rest of your flake
    }
    ```

#### Option 2: Cargo (Latest Release)

* Build and install via Cargo:
    ```sh
    $ cargo install fsel@3.7.0-kiwicrab
    ```
* To update later:
    ```sh
    $ cargo install fsel@3.7.0-kiwicrab --force
    ```
* Or install latest version (check [releases](https://github.com/Mjoyufull/fsel/releases)):
    ```sh
    $ cargo search fsel  # See available versions
    $ cargo install fsel@<version>
    ```

#### Option 3: AUR (Arch Linux)

* Install the git version with your favorite AUR helper:
    ```sh
    $ yay -S fsel-bin
    # or
    $ paru -S fsel-git
    ```
* Or manually:
    ```sh
    $ git clone https://aur.archlinux.org/fsel-bin.git
    $ cd fsel-bin
    $ makepkg -si
    ```
#### Option 4: Void linux (Unoffical Repo)

* Install fsel on void
    ```sh
    echo repository=https://raw.githubusercontent.com/Event-Horizon-VL/blackhole-vl/repository-x86_64 | sudo tee /etc/xbps.d/20-repository-extra.conf
    sudo xbps-install -S fsel
    ```
#### Option 5: Build from source

* Install [Rust](https://www.rust-lang.org/learn/get-started) stable
* Build:
    ```sh
    $ git clone https://github.com/Mjoyufull/fsel && cd fsel
    $ cargo build --release
    ```
* Copy `target/release/fsel` to somewhere in your `$PATH`
* (Optional) Create a dmenu symlink for drop-in compatibility:
    ```sh
    $ ./create_dmenu_symlink.sh
    ```
    Or manually: `ln -s $(which fsel) ~/.local/bin/dmenu`

### Optional Dependencies

* **uwsm** - Universal Wayland Session Manager (for `--uwsm` flag)
* **systemd** - For `--systemd-run` flag
* [**cclip**](https://github.com/heather7283/cclip) - Clipboard manager (for `--cclip` mode)
* **Kitty, Foot, WezTerm, or other Sixel/Kitty/Halfblocks-capable terminal** - For native inline image previews in cclip mode (powered by [ratatui-image](https://github.com/benjajaja/ratatui-image); no chafa needed in 3.1.0+)
* [**otter-launcher**](https://github.com/kuokuo123/otter-launcher) - Pairs nicely with fsel for a complete launcher setup see [Usage](./USAGE.md)

## Usage

### Interactive Mode

Run `fsel` from a terminal to open the interactive TUI launcher.

```sh
# Launch fsel
fsel

# Pre-fill search (must be last)
fsel -ss firefox

# Direct launch (no UI)
fsel -p firefox
```

**Highlights:**
- **Advanced Search Ranking**: Configurable scoring with `frecency`, `recency`, or `frequency`
- **Smart Matching**: Searches names, descriptions, keywords, and categories
- **Pin/Favorite Apps**: Press Ctrl-Space to pin apps - pinned apps always appear first
- **Reversible Hiding**: Press Alt-Delete to hide one exact launcher source and Alt-U to undo
- **Optional Deduplication**: Enable deterministic XDG-aware duplicate suppression when wanted
- **Match Modes**: Fuzzy (default) or exact matching

See [USAGE.md - App Launcher](./USAGE.md#app-launcher) for TTY mode, launch prefixes, `--detach`, cache management, `--replace`, and more.

Hidden entries are stored in fsel's database; their `.desktop` files and executables are not changed.
Manage persistent hides without opening the TUI:

```sh
fsel --list-hidden
fsel --unhide 12
fsel --unhide-all
```

Set `auto_hide_duplicates = true` under `[app_launcher]` to suppress entries with the same
desktop-file ID or normalized visible name. It defaults to `false`; manual hiding always targets
one exact source path, including duplicate entries in the same directory or across Bedrock strata.

### Direct Launch Mode

Launch applications directly from the command line without opening the TUI:

```sh
# Launch Firefox directly
fsel -p firefox

# Launch the best fuzzy match for "terminal" (default match mode)
fsel -p terminal

# Partial names work while match_mode is fuzzy
fsel -p fire  # Finds Firefox

# Exact mode requires an exact app or executable name
fsel --match-mode=exact -p firefox
fsel --match-mode=exact -p fire   # Fails: no exact match

# Combine with launch options
fsel --launch-prefix="runapp --" -p discord
fsel --uwsm -p discord
fsel --systemd-run -vv -p code
```

### Dmenu Mode

Fsel includes a full dmenu replacement mode that reads from stdin and outputs selections to stdout:

```sh
# Basic dmenu replacement
echo -e "Option 1\nOption 2\nOption 3" | fsel --dmenu

# Display only specific columns (like cut)
ps aux | fsel --dmenu --with-nth 2,11  # Show only PID and command

# Use custom delimiter
echo "foo:bar:baz" | fsel --dmenu --delimiter ":"

# Pipe from any command
ls -la | fsel --dmenu
find . -name "*.rs" | fsel --dmenu
git log --oneline | fsel --dmenu

# fzf-style preview command ({} is the selected row, passed via the environment)
find . -type f | fsel --preview 'file --brief {}'

# Preview image bytes with Kitty, Sixel, or the half-block fallback
find ~/Pictures -type f | fsel --preview 'cat {}'
```

See [USAGE.md - Dmenu Mode](./USAGE.md#dmenu-mode) for previews, column operations, password input, pre-selection, exact matching, `--dmenu0`, and prompt-only mode.

### Clipboard History Mode
Requires [cclip](https://github.com/heather7283/cclip).
<img width="853" height="605" alt="image" src="https://github.com/user-attachments/assets/0bf71952-f09a-4ce2-8807-bca1003c8daf" />

Browse and select from your clipboard history with image previews:

```sh
# Browse clipboard history with cclip integration
fsel --cclip

# Filter by tag
fsel --cclip --tag prompt

# List all tags
fsel --cclip --tag list

# List items with specific tag (verbose shows details)
fsel --cclip --tag list prompt -vv

# Clear tag metadata from fsel database
fsel --cclip --tag clear

# Show tag color names in display
fsel --cclip --cclip-show-tag-color-names
```

See [USAGE.md - Clipboard Mode](./USAGE.md#clipboard-mode) for tag management, keybindings, inline image details, and more clipboard-specific behavior.

### Command Line Help

```sh
# Quick overview grouped by mode/flags
fsel -h

# Full tree-style reference covering every option
fsel -H

# Show verbose output
fsel -vvv
```

See [USAGE.md](./USAGE.md) for more examples, launch methods, scripting recipes, debugging notes, [environment variables](./USAGE.md#environment-variables), and advanced usage.

## Configuration

Config file: `~/.config/fsel/config.toml`

### Basic Setup

```toml
# Colors
highlight_color = "LightBlue"
cursor = "█"

# App launcher
terminal_launcher = "alacritty -e"

# Pin/favorite settings (root-level UI options)
pin_color = "rgb(255,165,0)"       # Color for pin icon (orange)
pin_icon = "📌"                     # Icon for pinned apps

[app_launcher]
filter_desktop = true              # Filter apps by desktop environment
filter_actions = false            # Keep desktop actions visible; set true to hide them
auto_hide_duplicates = false      # Opt in to deterministic duplicate suppression
list_executables_in_path = false   # Show CLI tools from $PATH
match_mode = "fuzzy"               # "fuzzy" or "exact"
ranking_mode = "frecency"          # "frecency", "recency", or "frequency"
pinned_order = "ranking"           # "ranking", "alphabetical", "oldest_pinned", "newest_pinned"
```

Field placement matters. Root-level UI options and `[app_launcher]` / `[dmenu]` / `[cclip]` sections are validated separately.
See [config.toml](./config.toml) and [keybinds.toml](./keybinds.toml) for all options with detailed comments.
`[app_launcher].match_mode = "exact"` also applies to `-p/--program`, where it requires an exact app or executable name.

### Environment variable overrides

After the config file is loaded, any `FSEL_*` variable set in the process environment overrides the corresponding setting. Use this for one-off launches, wrappers, or systemd units without editing `config.toml`.

- **Root / shared keys:** `FSEL_` plus the uppercase TOML key (e.g. `match_mode` → `FSEL_MATCH_MODE`).
- **Section keys:** `FSEL_DMENU_*`, `FSEL_CCLIP_*`, and `FSEL_APP_LAUNCHER_*` mirror `[dmenu]`, `[cclip]`, and `[app_launcher]` fields.

```sh
FSEL_MATCH_MODE=exact fsel
FSEL_APP_LAUNCHER_FILTER_ACTIONS=true fsel
FSEL_HIGHLIGHT_COLOR=Cyan FSEL_DMENU_DELIMITER=: fsel --dmenu < items.txt
```

`man fsel` summarizes this under **ENVIRONMENT**. For the full variable list, see [Environment variables](./USAGE.md#environment-variables) in [USAGE.md](./USAGE.md).

### Window Manager Integration

**Sway/i3:**
```sh
# ~/.config/sway/config
set $menu alacritty --title launcher -e fsel
bindsym $mod+d exec $menu
for_window [title="^launcher$"] floating enable, resize set width 500 height 430, border none

# Clipboard history
bindsym $mod+v exec 'alacritty --title clipboard -e fsel --cclip'
```

**Hyprland:**
```sh
# ~/.config/hypr/hyprland.conf
bind = $mod, D, exec, alacritty --title launcher -e fsel
windowrule {
    match:title = launcher
    float = on 
    center = on 
    size = 500 430
}
```

**Niri:**
```sh
# ~/.config/niri/config.kdl
window-rule {
    match title="launcher"
    open-floating true
}

# Add inside binds { ... }
Mod+D { spawn "alacritty" "--title" "launcher" "-e" "fsel"; }
```

**dwm/bspwm/any WM:**
```sh
# Use dmenu mode
bindsym $mod+d exec "fsel --dmenu | xargs swaymsg exec --"
```

## Contributing

Contributions are welcome! Whether you're reporting bugs, suggesting features, or submitting code, we appreciate your help making fsel better.

### How to Contribute

1. **Bug Reports & Feature Requests**: Open an issue on [GitHub Issues](https://github.com/Mjoyufull/fsel/issues)
2. **Pull Requests**: Fork the repo, create a feature branch, and submit a PR
3. **Code Style**: Run `cargo fmt` and `cargo clippy` before submitting
4. **Testing**: Ensure `cargo test` and `cargo build --release` pass

### Development Workflow

See [CONTRIBUTING.md](./CONTRIBUTING.md) for detailed guidelines on:
- Branch naming conventions
- Commit message format
- Pull request process
- Code review standards
- Release procedures

All contributors are valued and appreciated. Your name will be added to the contributors list, and significant contributions will be highlighted in release notes.

Thank you for helping improve fsel!

## Philosophy

fsel is a **unified TUI workflow tool** built for terminal-centric setups. It combines app launching, dmenu functionality, and clipboard history into one scriptable interface with consistent keybinds and theming.

**This means:**
- It's built for my workflow first, but PRs for bug fixes and useful features are welcome as long as they fit in scope.
- Older versions and the original gyr exist if you want something more minimal.
---

## Troubleshooting

**Apps not showing up?**
- Check `$XDG_DATA_DIRS` includes `/usr/share/applications`
- Try `--filter-desktop=no` to disable desktop filtering
- Use `-vvv` for debug info

**Mouse not working?**
- Check your terminal supports mouse input
- Try `disable_mouse = false` in config

**Images not showing in cclip mode?**
- Use a Kitty-, Sixel-, or Halfblocks-capable terminal (e.g. Kitty, Foot, WezTerm). Image preview uses built-in [ratatui-image](https://github.com/benjajaja/ratatui-image); no chafa or other external viewer is needed (3.1.0+).
- Check `image_preview = true` in config
- Images render inside the content panel; press Alt+i for fullscreen preview

**Fuzzy matching too loose?**
- Try `--match-mode=exact` for stricter matching
- Or set `match_mode = "exact"` in config

**Too many desktop action entries?**
- Use `--filter-actions` to hide desktop actions like "New Window"
- Or set `filter_actions = true` under `[app_launcher]`

**Hidden the wrong launcher entry?**
- Press Alt-U before leaving the launcher to restore the most recent hide
- Run `fsel --list-hidden`, then `fsel --unhide <ID>` to restore a specific entry
- Run `fsel --unhide-all` to clear every manual hide

**Terminal apps not launching?**
- Set `terminal_launcher` in config
- Example: `terminal_launcher = "kitty -e"`

## Credits

Fork of [gyr](https://git.sr.ht/~nkeor/gyr) by Namkhai B.

## License

[BSD 2-Clause](./LICENSE) (c) 2020-2022 Namkhai B., Mjoyufull
