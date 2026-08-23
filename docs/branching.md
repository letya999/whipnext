# Branching

## Model

```
feature/*  →  dev  →  main
```

| Branch | Role |
|---|---|
| `dev` | Default. Integration branch. All work lands here first. |
| `main` | Protected release line. Receives **only** merges of current `dev`. |
| `feature/*` | Short-lived. Created **from `dev` only**. Merged back into `dev`. |

Never branch from `main`. Never commit directly to `main`. Never merge a feature into `main`.

## Daily work

```bash
git checkout dev
git pull
git checkout -b feature/short-name
# ... commits ...
git checkout dev
git merge --no-ff feature/short-name
git branch -d feature/short-name
```

Promote to `main` only when `dev` is the release candidate:

```bash
git checkout main
git merge --no-ff dev
git checkout dev
```

## Local protection

This repo uses `core.hooksPath=.githooks`.

After clone (already set if you ran `git init` in this tree):

```bash
git config core.hooksPath .githooks
```

Hooks:

- **pre-commit** — on `main`, only a merge whose `MERGE_HEAD` is the tip of `dev` is allowed.
- **pre-push** — rejects pushing `main` unless every new commit is reachable from `dev` *or* the push is a merge of `dev` (first-parent policy is documented here; the hook blocks any `main` update whose tip is not an ancestor of `dev` and is not exactly `dev` plus merge commits from `dev`).

If a hook is missing, do not bypass it with `--no-verify`.

## Hosted repo (GitHub / GitLab)

There is no remote yet. When you add one:

1. Push both branches, then set the **default branch to `dev`**.
2. Protect **`main`**: no direct pushes, no force-push, no deletions, required pull/merge request, and only from `dev` (or require reviews and restrict who can merge).
3. Optionally protect `dev` against force-push, but allow feature PRs.

GitHub example (adjust `OWNER/REPO`):

```bash
git remote add origin git@github.com:OWNER/REPO.git
git push -u origin dev
git push origin main
gh repo edit OWNER/REPO --default-branch dev
```

Then in the host UI: **Settings → Branches → Branch protection** for `main`.

GitLab: **Settings → Repository → Protected branches** — protect `main`, allowed to merge: maintainers, allowed to push: nobody. Default branch: `dev`.
