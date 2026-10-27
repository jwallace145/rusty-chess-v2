# Jimmy's Repo Conventions

Oct 14, 2026 · @Jimmy

## How to use this

Every new repo applies this whole document unless its own brief says otherwise. Hand it to coding agents together with the repo's brief, and say "apply the repo conventions". Requirements marked **MUST** are non-negotiable; an agent asks before deviating from one.

Export this doc as Markdown and commit it as `repo-conventions.md` in a dotfiles or templates repo, so changes to the standard are versioned like code. Placeholders look like `<REPO>`.

## Commit messages

**MUST:** every commit is a single-line Conventional Commit of at most 100 characters. Details go in the PR description, never in the commit body.

- Format: `type(scope)?: subject`, with an imperative, lowercase subject and no trailing period.
- Allowed after the header: a blank line followed by `Co-authored-by: Name <email>` trailers. Nothing else.
- Types: `feat`, `fix`, `docs`, `chore`, `refactor`, `ci`, `build`, `perf`, `style`, `test`, `revert`.
- Breaking changes are marked with `!`, as in `feat!: drop input foo`.
- The type drives Release Please: `feat` bumps the minor version, `fix` bumps the patch, and `!` bumps the major (or the minor while pre-1.0).

| Message | Valid |
| --- | --- |
| `feat(static-site): add security headers policy` | Yes |
| `fix: correct preview host regex` + blank line + `Co-authored-by: Claude <noreply@anthropic.com>` | Yes |
| `feat: add thing` + blank line + `This adds the thing because…` | No, body text |
| `Added thing` | No, not conventional |

## Commitlint

Use commitlint with `@commitlint/config-conventional` plus one local rule that allows only co-author trailers after the header. The built-in body and footer rules can't express that reliably.

```js
// commitlint.config.mjs
const TRAILER = /^Co-authored-by: .+ <.+>$/;

export default {
  extends: ['@commitlint/config-conventional'],
  plugins: [{
    rules: {
      'only-coauthor-trailers': ({ raw }) => {
        const extra = raw.trim().split('\n').slice(1)
          .filter((l) => l.trim() && !l.startsWith('#'));
        const bad = extra.filter((l) => !TRAILER.test(l.trim()));
        return [bad.length === 0,
          'commit must be one line; only Co-authored-by trailers may follow'];
      },
    },
  }],
  rules: {
    'header-max-length': [2, 'always', 100],
    'only-coauthor-trailers': [2, 'always'],
  },
};
```

- **Local:** a `commit-msg` hook through `pre-commit`, documented in `CONTRIBUTING.md`. It's a convenience only.
- **CI is authoritative:** lint every commit in the PR range (base..head), and lint the PR title separately, because squash merges use the title as the commit message.

## Pull requests

**MUST:** after the initial scaffold commit, every change reaches `main` through a PR. The PR description carries the detail that commits don't.

- The PR title is a valid Conventional Commit; CI lints it.
- Typed templates live in `.github/PULL_REQUEST_TEMPLATE/`: `feat.md`, `fix.md`, `docs.md`, `chore.md`, `refactor.md`, `ci.md`.
  - Every template has Summary, Motivation and context, What changed, How it was verified, and a Checklist (conventional title, one-line commits, docs updated).
  - `fix.md` adds Bug, Root cause, and Reproduction.
- `.github/pull_request_template.md` is the default. It links each typed template with `?template=<type>.md` and shows `gh pr create --template <type>.md`.

**Coding agents** follow the same rules, restated in `AGENTS.md`:

1. Never push to `main`; always work on a branch.
2. One-line conventional commits, with co-author trailers allowed.
3. Open PRs with `gh pr create --template <type>.md` and fill in every section.
4. Run the repo's checks locally before opening the PR.

## Branch protection and merging

**MUST:** PRs merge by squash or rebase only, with no merge commits and a linear history. Apply these settings with `gh api` once the conventions PR has merged.

| Setting | Value |
| --- | --- |
| Allow squash merging | On |
| Allow rebase merging | On |
| Allow merge commits | Off |
| Squash commit title | `PR_TITLE` |
| Squash commit message | `BLANK` (GitHub still adds co-author trailers) |
| Auto-delete head branches | On |

Branch ruleset on `main`:

- Require a pull request before merging
- Allowed merge methods: squash and rebase
- Require status checks: every CI job
- Require linear history
- Block force pushes and deletion
- No bypass actors

## CI baseline

Every repo's `ci.yml` runs on `pull_request` with these jobs, plus whatever build and test jobs the repo's brief adds.

| Job | Checks |
| --- | --- |
| commitlint | Every commit in base..head |
| pr-title | PR title is a Conventional Commit (`amannn/action-semantic-pull-request`) |
| repo-specific | Build, lint, test, as the repo's brief defines |

- Each job sets least-privilege `permissions:`.
- Actions are pinned to major versions, and Dependabot updates `github-actions` weekly.
- AWS access goes through GitHub OIDC roles, never long-lived keys. Role ARNs are stored as repository **variables**, not secrets.

## Releases with Release Please

Merging the Release Please PR is the human-reviewed gate for every release, whether that's a package, a module version, or an infrastructure apply. Release Please owns `CHANGELOG.md`; nobody edits it by hand.

- Release PRs are authored by the GitHub App `jwallace145-release-bot[bot]`, so CI runs on them. PRs opened with `GITHUB_TOKEN` don't trigger CI.
- Repo secrets: `RELEASE_BOT_CLIENT_ID` and `RELEASE_BOT_PRIVATE_KEY` (the full `.pem` contents).
- App permissions on each repo: Contents read/write and Pull requests read/write.
- Config uses a manifest (`release-please-config.json` and `.release-please-manifest.json`), `release-type: simple` unless an ecosystem type fits better, starting at `0.1.0`.
- Jobs that ship an artifact use `needs: release-please` and `if: needs.release-please.outputs.release_created == 'true'`.

```yaml
# .github/workflows/release-please.yml
on:
  push:
    branches: [main]
permissions: { contents: write, pull-requests: write }
jobs:
  release-please:
    runs-on: ubuntu-latest
    outputs:
      release_created: ${{ steps.rp.outputs.release_created }}
      tag_name: ${{ steps.rp.outputs.tag_name }}
    steps:
      - uses: actions/create-github-app-token@v2
        id: app-token
        with:
          client-id: ${{ secrets.RELEASE_BOT_CLIENT_ID }}
          private-key: ${{ secrets.RELEASE_BOT_PRIVATE_KEY }}
      - uses: googleapis/release-please-action@v4
        id: rp
        with:
          token: ${{ steps.app-token.outputs.token }}
          config-file: release-please-config.json
          manifest-file: .release-please-manifest.json
```

If `create-github-app-token` ever stops accepting `client-id`, switch to the input it documents for the app identifier.

## New repo checklist

Public repos use the MIT license, with the copyright holder taken from `git config user.name`. Every repo ships `README.md`, `CONTRIBUTING.md`, `AGENTS.md`, `LICENSE`, `.github/dependabot.yml`, and the PR templates.

Agent:

- [ ] Scaffold and push `chore: initial scaffold` to `main`. This is the only direct push, ever.
- [ ] Add commitlint, the pre-commit hook, PR templates, `AGENTS.md`, `CONTRIBUTING.md`, and CI, via a PR.
- [ ] Add the Release Please config and workflow, via a PR.
- [ ] Apply the merge settings and the `main` ruleset with `gh api`.
- [ ] Verify that a multi-line commit fails CI, a one-line commit with a co-author trailer passes, and a non-conventional PR title fails.

Owner:

- [ ] Install `jwallace145-release-bot` on the repo.
- [ ] Add the secrets `RELEASE_BOT_CLIENT_ID` and `RELEASE_BOT_PRIVATE_KEY`.
- [ ] Merge the first Release Please PR to cut `v0.1.0`.
