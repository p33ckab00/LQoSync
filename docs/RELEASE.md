# Versions, releases, and upgrades

This document gives the version number of lqosync3 and the release procedure. It also gives the install, upgrade, and rollback procedures on a shaper.

## Version number

The version has this form: `3.0.<commits>+g<hash>`.

| Part | Meaning | Example |
|---|---|---|
| `3` | Major version. LQoSync 3 follows the Python LQoSync 2.x. Increase it only for a change that breaks the config, the output, or the installation. | `3` |
| `0` | Minor version. Increase it by hand for a new set of features. | `0` |
| `<commits>` | The number of commits on the branch `lqosync3`. Each commit that you push increases it. A higher number is a newer version. | `117` |
| `+g<hash>` | The short commit hash. It names the exact source. | `+gb4f4031` |
| `.dirty` | The build had changes that are not committed. Do not install a dirty build on a shaper. | `.dirty` |
| `+unknown` | The build had no git history. | `3.0.0+unknown` |

The major and minor numbers are in `Cargo.toml` (`version = "3.0.0"`). `build.rs` sets the full version at build time.

See the version:

- On the shaper: `lqosync3 version`.
- In the TUI: the Versions card of the Dashboard shows the service version and the TUI version. If they are different, restart the service.
- In `/var/lib/lqosync3/status.json`: the key `program_version`.

### Find the source of a version

1. Read the hash after `+g`. Example: `3.0.117+gb4f4031` has the hash `b4f4031`.
2. Show that commit: `git show b4f4031`, or open `https://github.com/p33ckab00/LQoSync/commit/b4f4031`.
3. To find the commit from the number only: `git log --reverse --format=%h lqosync3 | sed -n 117p`.

### Rules that keep the number correct

- WARNING: DO NOT FORCE-PUSH OR REBASE THE BRANCH `lqosync3`. A changed history changes the commit numbers, and an old version number then names a different commit.
- Merge changes into `lqosync3` with a normal merge or a fast-forward. Do not squash a published range.
- The old Python tool has the tags `v2.x`. The tags of this tool start with `v3.`.

## Continuous integration

The workflow `.github/workflows/ci.yml` operates for each push and each pull request to `lqosync3`. It runs on Ubuntu 24.04:

- `cargo fmt --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --locked`
- `bash -n install.sh`
- a build with Rust 1.89 (the minimum Rust version)

Read the result on GitHub: the tab Actions, or the mark at the commit. Do not make a release from a commit with a failed check.

NOTE: GitHub Actions must be on for the repository: Settings, Actions, General, "Allow all actions".

## Make a release

Do this procedure on a computer with a clone of the repository. The release workflow does the build. It does not change a shaper.

1. Make sure that your clone is on `lqosync3`, with no changes that are not committed:

   ```bash
   git switch lqosync3
   git pull
   git status --short
   ```

   Expected result: no output from `git status --short`.

2. Make sure that the CI check of the last commit passed (tab Actions on GitHub).
3. Get the version number of the last commit:

   ```bash
   n=$(git rev-list --count HEAD)
   echo "v3.0.$n"
   ```

4. Make the tag on that commit and push it:

   ```bash
   git tag "v3.0.$n"
   git push origin "v3.0.$n"
   ```

5. Wait for the workflow `release` (tab Actions). It:
   - runs the tests,
   - builds the binary,
   - stops if the tag is not the version of the build, or if the build is dirty,
   - publishes a GitHub Release with `lqosync3-x86_64-linux`, `lqosync3-x86_64-linux.sha256`, `lqosync3.service`, and `install.sh`.
6. Make sure that the release shows on `https://github.com/p33ckab00/LQoSync/releases`.

If the tag does not agree with the version, delete the tag and make it again on the correct commit:

```bash
git tag -d "v3.0.$n"
git push origin ":refs/tags/v3.0.$n"
```

## Install or upgrade on a shaper

Do this procedure on the shaper, as a user with `sudo`. The installer needs no clone and no Rust.

1. Read the installer before you run it:

   ```bash
   curl -fsSL -o /tmp/install.sh https://raw.githubusercontent.com/p33ckab00/LQoSync/lqosync3/install.sh
   less /tmp/install.sh
   ```

2. Run it. Without `--version`, it installs the newest `v3` release:

   ```bash
   sudo bash /tmp/install.sh
   ```

   To install one specific release: `sudo bash /tmp/install.sh --version v3.0.120`.

3. Read the result:
   - A new installation writes `/etc/lqosync3/lqosync3.toml` with `mode = "shadow"` and no router. The check then fails, and the service does not start. Add the routers: `sudo lqosync3 tui`, tab 3, key `+`, then `v`, `A`, `y`. Then start the service: `sudo systemctl enable --now lqosync3.service`.
   - An upgrade keeps the config, the secrets, and the state. It replaces the binary and the unit file, runs `lqosync3 check`, and restarts the service.
4. Make sure that the service operates with the new version:

   ```bash
   lqosync3 version
   systemctl is-active lqosync3.service
   ```

5. Open the TUI. The Versions card must show the same version for the service and the TUI.

| Installer option | Effect |
|---|---|
| `--version v3.0.N` | Installs this release. Default: the newest `v3` release. |
| `--mode shadow\|apply` | `runtime.mode` of a new config file. It does not change an existing config file. Default: `shadow`. |
| `--no-start` | Does not start or restart the service. |

What the installer does not do:

- It does not change an existing config file or the secret files.
- It does not stop `lqosync-updatecsv.service` (the Python service). It gives a warning if that service is active.
- It does not change LibreQoS files, configuration, or services.
- It does not install packages, and it does not write to the routers.

CAUTION: THE RESTART STARTS A NEW CYCLE. IN `apply` MODE, THE CYCLE CAN WRITE THE LIBREQOS FILES AND RELOAD LIBREQOS.

## Roll back to an earlier release

1. Find the earlier release on the Releases page, for example `v3.0.118`.
2. Install it:

   ```bash
   sudo bash /tmp/install.sh --version v3.0.118
   ```

3. Make sure that `lqosync3 version` shows the earlier version.

NOTE: A rollback keeps the config. If a newer release added a config key that you set, the older binary stops with `unknown key`. Remove that key before the rollback, or run `lqosync3 check` after it.
