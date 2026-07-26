# starshell

A minimal, fast shell prompt with a fixed, hand-picked set of modules, built for Linux only.

> **This is a fork of [Starship](https://starship.rs).**
> starshell is a standalone fork with its own version history — it is **not** mechanically
> synced with upstream Starship releases. Upstream is consulted only as a manual reference
> when a change is worth porting into the core engine or into a module this project still
> ships. Most of Starship's modules have been intentionally removed; only twelve remain.
>
> All credit for the original design, engine, and module implementations goes to the
> [Starship contributors](https://github.com/starship/starship). starshell is distributed
> under the same [ISC License](https://opensource.org/licenses/ISC) as upstream.

## Why fork?

Starship supports dozens of modules covering nearly every language toolchain and cloud
provider. starshell strips that down to a fixed set of modules relevant to a single
Linux-only workflow, in exchange for a smaller binary, a simpler codebase, and no
cross-platform code paths to maintain.

## Modules

starshell ships exactly twelve modules — nothing else:

| Module | Purpose |
|---|---|
| `username` | Displays the active user's name. |
| `directory` | Displays the current working directory, with truncation and path substitution. |
| `git_branch` | Displays the active git branch. |
| `git_commit` | Displays the active commit hash / tag. |
| `git_state` | Displays the current git operation in progress (rebase, merge, etc.). |
| `git_status` | Displays working-tree status counts (staged, modified, untracked, ahead/behind). |
| `time` | Displays the current local time in a configurable format. |
| `character` | Displays the prompt character/arrow, styled by last command success/failure. |
| `os` | Displays an icon (and optionally name/version) for the detected OS/distro. Disabled by default. |
| `python` | Displays the active Python version / virtualenv name. |
| `line_break` | Inserts a line break in multi-line prompt formats. |
| `custom` | Runs a user-defined shell command and displays its output as a segment. |

No other modules exist in this codebase — there's nothing to disable, since unsupported
modules were removed entirely rather than gated off.

## Installation

### Prebuilt binary

```sh
curl -L https://github.com/AswinGopal/starshell/releases/latest/download/starshell -o starshell
chmod +x starshell
sudo mv starshell /usr/local/bin/
```

Prebuilt binaries are Linux-only (`x86_64`, built on `ubuntu-latest`) and are attached to each [GitHub Release](../../releases).

### Build from source

```sh
cargo build --release
```

The compiled binary will be at `target/release/starshell`.

## Usage

Add the following to your shell's config file to enable starshell, replacing `bash` with
the name of your shell:

```sh
eval "$(starshell init bash)"
```

Supported shells: `bash`, `zsh`, `fish`, `elvish`, `nushell`, `tcsh`, `xonsh`, `powershell`,
`ion`.

## Configuration

starshell reads its configuration from `starship.toml`, located via the `STARSHIP_CONFIG`
environment variable or the platform default config path — the same convention used by
upstream Starship, for drop-in compatibility with existing configs.

```sh
starshell config
```

opens the config file in `$EDITOR`.

## License

[ISC](https://opensource.org/licenses/ISC) — same as upstream Starship.
