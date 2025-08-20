# Welcome to the hugr-jeff development guide <!-- omit in toc -->

This guide is intended to help you get started with developing hugr-jeff.

If you find any errors or omissions in this document, please
[open an issue](https://github.com/cqcl/hugr-jeff/issues/new)!

## #️⃣ Setting up the development environment

You can setup the development environment in two ways:

### The Nix way

This repository defines a Nix flake which will allow you to quickly bootstrap a
development environment on any Linux system, including WSL & Mac OS X. Unlike
manually managing dependencies, Nix will (mostly) hermetically manage all the
dependencies for you. All you **need** to install, ever, is Nix itself.

To setup using nix flakes, you will first have to install
[Nix multi-user](https://nixos.org/download/), if you haven't done so already:

```bash
sh <(curl -L https://nixos.org/nix/install) --daemon
```

You can now trigger `nix develop` from the root of the repository and use the
development shell. You will have to
[enable flakes](https://wiki.nixos.org/wiki/Flakes) if you stop here. However,
installing `direnv` will enable nix development shell drop-in as soon as you
`cd` into the repository. This should be available with your favorite package
manager, e.g.

```bash
sudo apt-get install direnv
```

You will also have to add a `direnv` hook to your shell configuration, e.g.
`~/.bashrc`

```bash
eval "$(direnv hook bash)"
```

Refer to `direnv` install instructions for more help:

- [Installation](https://direnv.net/docs/installation.html)
- [Hook](https://direnv.net/docs/hook.html)

Alternatively, if you already use `nix-darwin`, `home-manager`, etc. you can
enable direnv in your config, e.g.:

```
{
  ...
  outputs = inputs@{ self, nix-darwin, nixpkgs }:
    let
        configuration = { pkgs, ... }: {
            programs.direnv.enable = true;
        }
    ...
}
```

Once you have both nix and `direnv` installed, you will have to `direnv allow`
in the repository root as a one-time step to allow `direnv` to trigger
`nix develop` for you. Nix will now manage the toolchain and dev environment for
you.

> [!NOTE]
>
> Unfortunately, Mac OS X also requires XCode tooling to be installed and
> configured externally. While `darwin.xcode_XX` packages exist, they require
> manual download and provide little to no benefit over managing externally.

### Manual setup

To setup the environment manually you will need:

- Rust `1.85.0`: https://www.rust-lang.org/tools/install
- Optional: Just: https://just.systems/
- Optional: uv `0.6.10`: docs.astral.sh/uv/getting-started/installation

Once you have these installed, you can optionally setup pre-commit hooks with:

```bash
just setup
```

## 🏃 Running the tests

To compile and test the code, run:

```bash
just test
```

Run the rust benchmarks with:

```bash
cargo bench
```

Run `just` to see all available commands.

### 💥 API-breaking changes

Any breaking change in the public Rust APIs will cause the next release to be a
major version bump. You can check the next release version
[draft release PR](https://github.com/cqcl/hugr-jeff/pulls?q=is%3Aopen+is%3Apr+label%3Arelease)
on github.

Use `cargo semver-checks` to alert you of any problematic changes. Replace the
baseline-rev with a commit hash if your branch is not up-to-date with the main
branch.

```bash
# Ensure you have cargo-semver-checks installed
cargo install cargo-semver-checks --locked
# Check for breaking changes against the main branch
cargo semver-checks --baseline-rev origin/main
```

These checks are also run on the CI. You will see a warning comment on your PR
if you introduce a breaking change.

## 💅 Coding Style

The rustfmt tool is used to enforce a consistent rust coding style. The CI will
fail if the code is not formatted correctly.

To format your code, run:

```bash
just format
```

We also use various linters to catch common mistakes and enforce best practices.
To run these, use:

```bash
just check
```

To quickly fix common issues, run:

```bash
just fix
```

## 🌐 Contributing to hugr-jeff

We welcome contributions to hugr-jeff! Please open
[an issue](https://github.com/cqcl/hugr-jeff/issues/new) or
[pull request](https://github.com/cqcl/hugr-jeff/compare) if you have any
questions or suggestions.

PRs should be made against the `main` branch, and should pass all CI checks
before being merged. This includes using the
[conventional commits](https://www.conventionalcommits.org/en/v1.0.0/) format
for the PR title.

The general format of a contribution title should be:

```
<type>(<scope>)!: <description>
```

Where the scope is optional, and the `!` is only included if this is a semver
breaking change that requires a major version bump.

We accept the following contribution types:

- feat: New features.
- fix: Bug fixes.
- docs: Improvements to the documentation.
- style: Formatting, missing semi colons, etc; no code change.
- refactor: Refactoring code without changing behaviour.
- perf: Code refactoring focused on improving performance.
- test: Adding missing tests, refactoring tests; no production code change.
- ci: CI related changes. These changes are not published in the changelog.
- chore: Updating build tasks, package manager configs, etc. These changes are
  not published in the changelog.
- revert: Reverting previous commits.
