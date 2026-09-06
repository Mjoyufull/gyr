# fsel Usage Guide

Quick reference for common use cases.

## App Launcher

### Basic Usage
```sh
# Launch fsel
fsel

# Pin your favorite apps (Ctrl+Space in TUI)
# Pinned apps always appear first with 📌 icon

# Configure app ranking in [app_launcher] with:
# ranking_mode = "frecency" | "recency" | "frequency"
# pinned_order = "ranking" | "alphabetical" | "oldest_pinned" | "newest_pinned"

# Pre-fill search (works with app launcher, dmenu, and cclip modes)
# Note: -ss must be the LAST option
fsel -ss firefox

# Direct launch (no UI)
fsel -p firefox

# Print JSON to stdout (no UI)
fsel --stdout -ss firefox

# Use custom config
fsel -c ~/.config/fsel/test-config.toml
fsel --config ~/.config/fsel/test-config.toml

# Hide list until typing
fsel --hide-before-typing

# Show CLI tools from $PATH
fsel --list-executables-in-path

# Hide list until typing
fsel --hide-before-typing

# Exact matching only
fsel --match-mode=exact

# Direct launch also respects match mode
fsel --match-mode=exact -p firefox
fsel --match-mode=exact -p fire   # Fails: no exact match

# Cache management
fsel --clear-cache      # Clear all caches (full rebuild)
fsel --refresh-cache    # Refresh file list (pick up new apps)
fsel --clear-history    # Clear launch history

# Replace existing instances (fsel and cclip modes only)
fsel -r                 # Replace running fsel instance (ensures previous session exits)
fsel --cclip -r         # Replace running cclip instance
# Not supported in --dmenu mode
```

### Hidden Entries

Press `Alt+Delete` to hide the exact selected launcher entry. This records its source in fsel's
database; it does not delete or edit the desktop file or executable. Press `Alt+U` to restore the
most recently hidden entry.

```sh
# Show persistent manual hide records and their numeric IDs
fsel --list-hidden

# Restore one record
fsel --unhide 12

# Restore every manually hidden entry
fsel --unhide-all
```

Hides apply to the interactive launcher, direct launch, and `--stdout`. Entries with the same
visible name remain independent when they come from different source paths. Clearing history or
the desktop cache does not clear hidden-entry records.

Automatic duplicate suppression is separate from manual hides and defaults to off:

```toml
[app_launcher]
auto_hide_duplicates = false
```

Set it to `true`, or run `fsel --auto-hide-duplicates`, to keep one deterministic entry for equal
desktop-file IDs and equal normalized names. `$XDG_DATA_HOME` wins first, followed by each
`$XDG_DATA_DIRS` entry in order. Automatic exclusions are not written to the database. Manually
hiding the current winner exposes the next eligible source, which keeps the behavior usable on
Bedrock Linux and for duplicates inside the same application directory.

### Launch Methods
```sh
# Default (direct execution)
fsel

# TTY mode: Launch terminal applications in the current terminal session.
# In TTY mode fsel replaces itself with the selected terminal program (exec),
# so the launched app takes over the current terminal (useful for htop, vim, etc).
# Enable with -t or --tty, or set `terminal_launcher = "tty"` in config.
fsel -t
fsel --tty

# Through a custom launcher prefix
fsel --launch-prefix="runapp --"

# Through systemd
fsel --systemd-run
fsel --systemd-run --detach   # Fully detached using systemd scope

# Through uwsm
fsel --uwsm
fsel --uwsm --detach          # Fully detached via uwsm

# Detach from terminal (prevents apps from being killed when terminal closes)
# Useful for apps like Discord or Steam; works standalone or with launch prefixes,
# --systemd-run, or --uwsm
fsel -d
fsel --detach

# Print command instead of running
fsel --no-exec
```

## Dmenu Mode

### Basic Dmenu
```sh
# Simple selection
echo -e "Option 1\nOption 2\nOption 3" | fsel --dmenu

# From file
cat options.txt | fsel --dmenu

# From command output
git branch | fsel --dmenu

# Null-separated input
find . -print0 | fsel --dmenu0
```

### Column Operations
```sh
# Display only column 2
ps aux | fsel --dmenu --with-nth=2

# Display column 2, output column 1
ps aux | fsel --dmenu --with-nth=2 --accept-nth=1

# Match against column 3, display column 1
printf "A\tB\tC\nD\tE\tF" | fsel --dmenu --with-nth=1 --match-nth=3

# Custom delimiter
echo "A:B:C" | fsel --dmenu --delimiter=":"
```

### Special Modes
```sh
# Password input
echo -e "pass1\npass2" | fsel --dmenu --password

# Custom password character
echo -e "pass1\npass2" | fsel --dmenu --password=•

# Output index instead of text
echo -e "A\nB\nC" | fsel --dmenu --index

# Prompt-only (no list)
fsel --dmenu --prompt-only

# Force selection from list
echo -e "A\nB\nC" | fsel --dmenu --only-match
```

### Pre-selection
```sh
# Pre-fill search query
echo -e "firefox\nchrome\nfirefox-dev" | fsel --dmenu -ss fire

# Pre-select by string
git branch | fsel --dmenu --select main

# Pre-select by index
echo -e "A\nB\nC" | fsel --dmenu --select-index=1

# Auto-select when one match
echo -e "Option 1\nOption 2" | fsel --dmenu --auto-select
```

### Matching
```sh
# Exact matching
echo -e "test\ntesting\ntest123" | fsel --dmenu --match-mode=exact

# Exit if empty input
cat empty.txt | fsel --dmenu --exit-if-empty
```

## Clipboard Mode

### Basic Usage
```sh
# Browse clipboard history
fsel --cclip

# Pre-fill search to find specific content
fsel --cclip -ss image

# With image previews (Kitty, Sixel, or Halfblocks-capable terminal; 3.1.0+ uses built-in ratatui-image, no chafa)
fsel --cclip  # Images show automatically if supported
```

### Content Preview Diagnostics
```sh
# Show raw textual content instead of rendering HTML as readable text
fsel --cclip -v

# Also identify the selected row ID, MIME type, and active preview view
fsel --cclip -vvv

# Show raw HTML for inspection, but copy its rendered text when selected
fsel --cclip -vx

# Verbosity remains independent from rendered copying
fsel --cclip -vvvvx
```

Binary clipboard payloads are never printed as text. Image entries continue to use the terminal
image preview when supported. `-x` changes only HTML copying: rendered HTML is offered as
`text/plain;charset=utf-8`; plain text, images, and other MIME types retain the original copy path.
`wl-copy` is used when available, with `cclip copy -` as the rendered-text fallback. Interactive
HTML previews load complete content only for selected entries; noninteractive tagged listings also
hydrate otherwise blank truncated HTML. Text decoding honors declared MIME charsets, replaces
invalid byte sequences for display, preserves `<pre>` whitespace, and decodes standard named
entities. Empty clipboard payloads remain valid on both original and rendered copy paths.

### Tag Management
```sh
# Filter clipboard items by tag
fsel --cclip --tag prompt
fsel --cclip --tag code

# List all available tags
fsel --cclip --tag list

# List items with specific tag
fsel --cclip --tag list prompt

# List items with tag (verbose shows details)
fsel --cclip --tag list prompt -vv

# Clear tag metadata from fsel database
# Note: This only clears fsel's tag metadata (colors, emojis)
# To clear tags from cclip entries, use: cclip tag -d <ID>
fsel --cclip --tag clear

# Wipe ALL tags from cclip entries (requires cclip 3.2+)
# This removes the tag text (e.g., "[tag]") from the actual clipboard items
fsel --cclip --tag wipe

# Show tag color names in item display
fsel --cclip --cclip-show-tag-color-names
```

### Keybindings in cclip mode
- `Enter` - Copy selection to clipboard
- `Alt+i` - Display image fullscreen (bypass TUI)
- `Alt+Delete` - Delete selected clipboard entry (selection stays at the same physical index; next item becomes selected)
- `Esc` - Exit without copying
- Arrow keys - Navigate
- Type to filter

**Note:** Tag creation and management requires cclip with tag support. Tags appear as `[tagname]` prefixes in the clipboard item list.

## Scripting Examples

### Process Killer
```sh
ps aux | fsel --dmenu --with-nth=2,11 --accept-nth=2 | xargs kill
```

### Git Branch Switcher
```sh
git branch | fsel --dmenu --select main | xargs git checkout
```

### SSH Connection Picker
```sh
grep "^Host " ~/.ssh/config | fsel --dmenu --with-nth=2 | xargs ssh
```

### Window Switcher (Sway)
```sh
swaymsg -t get_tree | \
  jq -r '..|select(.type=="con" and .name!=null)|.name' | \
  fsel --dmenu | \
  xargs -I {} swaymsg '[title="{}"] focus'
```

## Tips & Tricks

### Terminal Recommendations

**Best:** Kitty - Full inline image support, best performance
```sh
# Install Kitty
sudo pacman -S kitty  # Arch
sudo apt install kitty  # Debian/Ubuntu
```

**Also Great:** Foot, WezTerm, any Sixel-capable terminal
- Sixel now fully supported for inline previews

### Drop-in dmenu Replacement
```sh
# Create symlink
ln -s $(which fsel) ~/.local/bin/dmenu

# Now scripts using dmenu will use fsel
rofi-script.sh  # Works automatically
```

### Otter-Launcher Integration

Combine fsel with [otter-launcher](https://github.com/kuokuo123/otter-launcher) for a powerful dual-mode setup:

**Setup:**
1. Typing just an app name → Opens fsel with pre-filled search
2. Typing `app <name>` → Instantly launches app without TUI

```toml
# ~/.config/otter-launcher/config.toml
[general]
default_module = "search"
empty_module = "search"
exec_cmd = "sh -c"

# Mode 1: Search mode (default)
[[modules]]
description = "search apps with fsel"
prefix = "search"
cmd = "fsel -vv -d -r -ss \"{}\""
with_argument = true

# Mode 2: Instant launch
[[modules]]
description = "launch apps instantly"
prefix = "app"
cmd = "fsel -vv -d -r -p \"{}\""
with_argument = true
```

**Usage:**
```sh
# In otter-launcher:
firefox          # Opens fsel with "firefox" pre-searched
app firefox      # Instantly launches Firefox (no TUI)
app code         # Instantly launches VS Code
```

**Optional: Add launch method flags if needed:**
```toml
# With a custom launcher prefix
cmd = "fsel --launch-prefix='runapp --' -vv -d -r -p \"{}\""

# With uwsm (requires uwsm installed)
cmd = "fsel --uwsm -vv -d -r -p \"{}\""

# With systemd-run (requires systemd)
cmd = "fsel --systemd-run -vv -d -r -p \"{}\""

```

**Warning:** Keep `unbind_proc` disabled for Fsel modules whilst using -d, and you need to do -d for apps to launch without unbind_proc and you need unbind_proc to launch apps without -d,. If it is set to `true`, otter-launcher returns to its own prompt while `fsel` is still running and raw terminal input will leak (escape sequences like `[B`). Use a dedicated terminal wrapper if you need asynchronous launch behavior.
```

### Performance with Large Lists
```sh
# Disable desktop filtering for speed
fsel --filter-desktop=no

# Use exact matching for faster results
fsel --match-mode=exact

# Limit executables from PATH
# (edit config to disable list_executables_in_path)
```

### Debug/Test Mode

Enable detailed debug logging with `-T` or `--test`:

```sh
# Enable debug mode
fsel -T

# Debug logs are written to ~/.config/fsel/logs/
# Filename format: fsel-debug-YYYYMMDD-HHMMSS-pidXXXXX.log
```

**What gets logged:**
- Startup configuration (prefix_depth, match_mode, ranking_mode, pinned_order, etc.)
- Total apps loaded and frecency entries
- Query changes (each character typed, backspace)
- Search snapshots with full scoring breakdown:
  - Tier classification (Pinned App Name Exact, Normal Fuzzy Match, etc.)
  - Bucket score, matcher score, ranking boost
  - Note: the user-facing `ranking boost` appears in logs as the active ranking label, e.g. `frecency: 0.500`.
  - Top 50 matches with complete breakdown
  - Filter timing
- Selection changes (which app is selected, scroll position)
- Launch events (app name, command, scoring details)
- Session summary (total duration)

**Use cases:**
- Debug why certain apps rank higher/lower than expected
- Understand search ranking algorithm behavior
- Analyze performance (filter timing)
- Track user interaction patterns
- Verify prefix_depth and other configuration settings

**Example log output:**
```
=== FSEL DEBUG SESSION STARTED ===
Timestamp: 2026-02-02 14:30:45.123
PID: 12345
Version: 3.6.0-kiwicrab
Log file: /home/user/.config/fsel/logs/fsel-debug-20260202-143045-pid12345.log

[STARTUP] Configuration:
  Prefix depth: 3
  Match mode: Fuzzy
  ...

[QUERY] User typed 'f': "" -> "f"
[SEARCH] Query: "f" (len: 1, prefix_depth: 3)
[SEARCH] Filter time: 2ms
[SEARCH] Total matches: 45 (showing top 50)
  [  1] Firefox (Score: 90000050)
       ├── Tier: Normal App Name Exact
       ├── Bucket Score: 90000000
       ├── Matcher Score: 50 (base: 0, 100x multiplier)
       └── frecency: 0.500 (raw: 0.500, boost: +5)
...
```

### Prefix Depth Configuration

The `prefix_depth` setting controls how many characters must be typed before prefix matching gets priority over fuzzy matching:

```sh
# Set prefix depth via CLI
fsel --prefix-depth 5

# Or in config.toml
prefix_depth = 5
```

**How it works:**
- When query length ≤ prefix_depth: Prefix matches (word-start, exact, etc.) get higher priority
- When query length > prefix_depth: All matches use fuzzy scoring equally
- Default: 3 characters

**Example:**
- With `prefix_depth = 3`:
  - Typing "fi" (2 chars): Prefix matches prioritized
  - Typing "fire" (4 chars): Fuzzy matching takes over
- With `prefix_depth = 5`:
  - Typing "fire" (4 chars): Still uses prefix priority
  - Typing "firef" (5 chars): Prefix priority
  - Typing "firefo" (6 chars): Fuzzy matching

### Debugging
```sh
# Quick overview grouped by mode/flags
fsel -h

# Full tree-style reference covering every option
fsel -H

# Show verbose output
fsel -vvv
```

## Configuration

### Config File Structure

Configuration is stored in `~/.config/fsel/config.toml`. **Field placement is critical** - putting options in the wrong section will cause crashes.

#### Correct Structure:
```toml
# Root level - UI/Color options go here
highlight_color = "LightBlue"
main_border_color = "White"
pin_color = "Orange"
terminal_launcher = "kitty -e"  # or "tty" for TTY mode (-t/--tty)

# App launcher specific options
[app_launcher]
filter_desktop = true
filter_actions = false
auto_hide_duplicates = false
list_executables_in_path = false
ranking_mode = "frecency"
pinned_order = "ranking"

# Dmenu mode overrides
[dmenu]
delimiter = " "
show_line_numbers = true

# Clipboard mode overrides  
[cclip]
image_preview = true
```

### Environment variables

Settings are loaded in this order: **built-in defaults** → **`config.toml`** → **`FSEL_*` environment variables** (env wins when set).

Variable names use the `FSEL_` prefix and match config field names in `SCREAMING_SNAKE_CASE`. Section-specific options use an extra infix:

| Config area | Prefix | Example TOML key | Environment variable |
|-------------|--------|------------------|----------------------|
| Root / general | `FSEL_` | `match_mode` | `FSEL_MATCH_MODE` |
| `[dmenu]` | `FSEL_DMENU_` | `delimiter` | `FSEL_DMENU_DELIMITER` |
| `[cclip]` | `FSEL_CCLIP_` | `image_preview` | `FSEL_CCLIP_IMAGE_PREVIEW` |
| `[app_launcher]` | `FSEL_APP_LAUNCHER_` | `ranking_mode` | `FSEL_APP_LAUNCHER_RANKING_MODE` |

**Types:** Use `true` / `false` for booleans and decimal integers for numeric fields (e.g. `FSEL_PREFIX_DEPTH=3`). If parsing fails, fsel reports an invalid environment override and exits. Strings (colors, modes, paths) are taken as-is.

**`FSEL_APP_LAUNCHER_LAUNCH_PREFIX`:** Parsed like shell words (quoted segments allowed), same idea as a command-line prefix.

```sh
export FSEL_RANKING_MODE=recency
fsel

# One-shot
FSEL_FILTER_DESKTOP=false FSEL_MATCH_MODE=exact fsel -p nvim

# Hide desktop action entries for launcher mode only
FSEL_APP_LAUNCHER_FILTER_ACTIONS=true fsel

# Suppress duplicate launcher entries for one invocation
FSEL_APP_LAUNCHER_AUTO_HIDE_DUPLICATES=true fsel
```

Note: Bare `FSEL_*` launcher keys set root defaults. `[app_launcher]` in `config.toml` or
`FSEL_APP_LAUNCHER_*` overrides them for the app launcher. `filter_actions` and
`auto_hide_duplicates` are launcher-only.

**General / launcher (root-level and shared launcher behavior):**

`FSEL_TERMINAL_LAUNCHER`, `FSEL_FILTER_DESKTOP`, `FSEL_LIST_EXECUTABLES_IN_PATH`, `FSEL_HIDE_BEFORE_TYPING`, `FSEL_MATCH_MODE`, `FSEL_RANKING_MODE`, `FSEL_PINNED_ORDER`, `FSEL_SYSTEMD_RUN`, `FSEL_UWSM`, `FSEL_DETACH`, `FSEL_NO_EXEC`, `FSEL_CONFIRM_FIRST_LAUNCH`, `FSEL_PREFIX_DEPTH`

**Default UI / layout (applies when a mode does not override):**

`FSEL_HIGHLIGHT_COLOR`, `FSEL_CURSOR`, `FSEL_HARD_STOP`, `FSEL_ROUNDED_BORDERS`, `FSEL_DISABLE_MOUSE`, `FSEL_TITLE_PANEL_HEIGHT_PERCENT`, `FSEL_INPUT_PANEL_HEIGHT`, `FSEL_TITLE_PANEL_POSITION`

**`[dmenu]` overrides (`FSEL_DMENU_*`):**

`DELIMITER`, `PASSWORD_CHARACTER`, `SHOW_LINE_NUMBERS`, `WRAP_LONG_LINES`, `EXIT_IF_EMPTY`, `DISABLE_MOUSE`, `HARD_STOP`, `ROUNDED_BORDERS`, `CURSOR`, `HIGHLIGHT_COLOR`, `MAIN_BORDER_COLOR`, `ITEMS_BORDER_COLOR`, `INPUT_BORDER_COLOR`, `MAIN_TEXT_COLOR`, `ITEMS_TEXT_COLOR`, `INPUT_TEXT_COLOR`, `HEADER_TITLE_COLOR`, `TITLE_PANEL_HEIGHT_PERCENT`, `INPUT_PANEL_HEIGHT`, `TITLE_PANEL_POSITION` (each prefixed with `FSEL_DMENU_`)

**`[cclip]` overrides (`FSEL_CCLIP_*`):**

`IMAGE_PREVIEW`, `HIDE_INLINE_IMAGE_MESSAGE`, `SHOW_TAG_COLOR_NAMES`, `SHOW_LINE_NUMBERS`, `WRAP_LONG_LINES`, `DISABLE_MOUSE`, `HARD_STOP`, `ROUNDED_BORDERS`, `CURSOR`, `HIGHLIGHT_COLOR`, `MAIN_BORDER_COLOR`, `ITEMS_BORDER_COLOR`, `INPUT_BORDER_COLOR`, `MAIN_TEXT_COLOR`, `ITEMS_TEXT_COLOR`, `INPUT_TEXT_COLOR`, `HEADER_TITLE_COLOR`, `TITLE_PANEL_HEIGHT_PERCENT`, `INPUT_PANEL_HEIGHT`, `TITLE_PANEL_POSITION` (each prefixed with `FSEL_CCLIP_`)

**`[app_launcher]` overrides (`FSEL_APP_LAUNCHER_*`):**

`FILTER_DESKTOP`, `FILTER_ACTIONS`, `LIST_EXECUTABLES_IN_PATH`, `HIDE_BEFORE_TYPING`, `LAUNCH_PREFIX`, `MATCH_MODE`, `RANKING_MODE`, `PINNED_ORDER`, `CONFIRM_FIRST_LAUNCH`, `PREFIX_DEPTH` (each prefixed with `FSEL_APP_LAUNCHER_`)

Keybinds are not configurable via environment variables; use `~/.config/fsel/keybinds.toml` or the `[keybinds]` section in `config.toml`. When both are present, the embedded `[keybinds]` section takes precedence.

#### Common Mistakes (Will Crash):
```toml
# WRONG - Color options in app_launcher section
[app_launcher]
main_border_color = "White"  # This will crash!
filter_desktop = true
filter_actions = true

# WRONG - App launcher options at root level
filter_desktop = true  # This should be in [app_launcher]
filter_actions = true  # This should be in [app_launcher]
```

### Error Messages

If you see errors like:
```
Error reading config file: unknown field `pin_color`, expected one of `filter_desktop`, `list_executables_in_path`...
```

This means you've placed a **color/UI option inside the [app_launcher] section**. Move it to the root level.

### Field Reference

**Root Level Fields:**
- Colors: `highlight_color`, `main_border_color`, `apps_border_color`, `input_border_color`, `main_text_color`, `apps_text_color`, `input_text_color`, `header_title_color`, `pin_color`
- UI: `cursor`, `rounded_borders`, `hard_stop`, `fancy_mode`, `pin_icon`, `disable_mouse`
- Layout: `title_panel_height_percent`, `input_panel_height`, `title_panel_position`
- General: `terminal_launcher` (use `"tty"` for TTY mode, same as -t/--tty), `keybinds`

**[app_launcher] Section (strict validation):**
- `filter_desktop`, `filter_actions`, `auto_hide_duplicates`, `list_executables_in_path`, `hide_before_typing`, `match_mode`, `ranking_mode`, `pinned_order`, `confirm_first_launch`, `prefix_depth`

**[dmenu] Section:**
- Colors: `highlight_color`, `main_border_color`, `items_border_color`, `input_border_color`, `main_text_color`, `items_text_color`, `input_text_color`, `header_title_color`
- UI: `cursor`, `hard_stop`, `rounded_borders`, `disable_mouse`
- Layout: `title_panel_height_percent`, `input_panel_height`, `title_panel_position`
- Parsing: `delimiter`, `show_line_numbers`, `wrap_long_lines`
- Behavior: `password_character`, `exit_if_empty`

**[cclip] Section:**
- Colors: `highlight_color`, `main_border_color`, `items_border_color`, `input_border_color`, `main_text_color`, `items_text_color`, `input_text_color`, `header_title_color`
- UI: `cursor`, `hard_stop`, `rounded_borders`, `disable_mouse`
- Layout: `title_panel_height_percent`, `input_panel_height`, `title_panel_position`
- Display: `show_line_numbers`, `wrap_long_lines`
- Images: `image_preview`, `hide_inline_image_message`
