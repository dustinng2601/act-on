# act-on

> Run GitHub Actions on your own devices + GitHub CI / enterprise pools.
> Cross-platform (Windows / Linux / macOS) local sandbox runner with
> policy-based device assignment.
> Rust implementation inspired by [nektos/act](https://github.com/nektos/act).

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

`act-on` lets you run GitHub Actions workflows locally and across your own
fleet of devices. It is the next step beyond `nektos/act`: not just "run a
workflow on my laptop", but "run the right job on the right machine, falling
back to GitHub-hosted or enterprise-shared CI where a device is missing".

## Why?

`nektos/act` proved you can run GitHub Actions locally. `act-on` extends that
to real-world enterprise fleets:

- **Staff A** has 1 MacBook and 2 Linux servers at home. macOS CI runs on the
  MacBook, Linux CI runs on the Linux servers, and Windows CI runs on
  GitHub-hosted runners (or a shared enterprise pool).
- **Staff B** has only one Linux box. When a job needs macOS, `act-on` looks
  for an available macOS device in the enterprise pool, or schedules the job
  on GitHub CI.
- **Staff C** has no devices at all. All of their jobs run on GitHub CI or on
  the public enterprise pool.

## Features

- **Cross-platform**: Windows, Linux, macOS first-class. Each platform has its
  own sandbox profile that tries to match GitHub-hosted runner images.
- **Sandbox execution**: each `run:` step is executed in a sandbox that
  replicates GitHub's CI environment (`GITHUB_*` env vars, `RUNNER_*`,
  workspace layout, file-command files `GITHUB_OUTPUT` / `GITHUB_PATH` /
  `GITHUB_ENV` / `GITHUB_STATE`).
- **Device pool**: declare which devices belong to whom and how they can be
  shared. Missing platforms are transparently routed to GitHub CI or to a
  shared enterprise pool.
- **Workflow events**: `push`, `pull_request`, `workflow_dispatch`
  (auto-trigger or manual), `schedule`, `repository_dispatch`.
- **Pass / fail status**: every step and job gets a clear
  `success` / `failure` / `skipped` outcome, with optional `continue-on-error`.
- **Expression evaluation**: full GitHub Actions expression language
  (`${{ ... }}`, context refs, operators, and the built-in functions
  `success()`, `failure()`, `cancelled()`, `always()`, `contains()`,
  `startsWith()`, `endsWith()`, `format()`, `join()`, `toJSON()`,
  `fromJSON()`, `hashFiles()`).
- **Actions**: `uses: org/repo@ref`, `uses: ./local`, `uses: docker://image`,
  composite actions, and reusable workflows.
- **Policy engine**: write `policy.yml` to define who owns which device, when
  the device can be borrowed by the pool, and what fallback strategy to use
  (local-first, pool-first, github-first).
- **Single fast binary**: pure Rust, async via `tokio`, optimized release
  profile (`lto = "fat"`, `codegen-units = 1`).

## Getting started

```console
$ cargo install --path .
$ act-on --help
$ act-on -W .github/workflows/ci.yml --job build -e push
```

Example `policy.yml`:

```yaml
version: 1
owner: staff-a
devices:
  - id: macbook-pro
    os: macos
    arch: arm64
    labels: [self-hosted, macos, arm64]
    share: pool        # available to enterprise pool when idle
  - id: linux-server-1
    os: linux
    arch: x86_64
    labels: [self-hosted, linux, x64]
    share: pool
  - id: linux-server-2
    os: linux
    arch: x86_64
    labels: [self-hosted, linux, x64]
    share: pool
fallback:
  missing_platform: github   # use GitHub CI when no local+pool device matches
  pool_endpoint: https://ci.enterprise.tld
```

## Roadmap

- [x] Workflow & action model
- [x] Expression evaluator
- [x] Local sandbox step execution (`run:` steps)
- [x] Workflow command & file-command protocol
- [x] Device pool + policy-based routing
- [x] `actions/checkout` local short-circuit
- [ ] Docker container runner (`docker://`, `Dockerfile`)
- [ ] Node.js action runtime (`node20+`)
- [ ] Composite + reusable workflow execution (model in place, full impl soon)
- [ ] GitHub CI / enterprise pool webhook transport
- [ ] TUI dashboard for live fleet overview

## License

[MIT](LICENSE) - free for everyone, not tied to SoundSeek internal tooling.
