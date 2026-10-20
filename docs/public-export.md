# Public export boundary

This cleanup does not authorize a merge, installation, deployment, repository
creation, visibility change, credential change, or history rewrite. The private
repository and all of its existing commits, branches, PR refs, releases, and
deployment identity remain private. A clean worktree is insufficient evidence
for publishing those objects.

## External configuration contract

The launcher accepts `HUNDRED_SERVER` and `HUNDRED_TOKEN_PASS_ENTRY` from a private
`launcher.env` outside the checkout. `HUNDRED_CONFIG` selects its path. Environment
values take precedence; `HUNDRED_TOKEN` belongs only in the process environment.
The pass mapping file is not an automatic input. Use `pass-env run -f` with your
external file when a command needs a provider or client credential. Synthetic
examples are in `config/`; no real mapping is required to build or verify.

Keep the Worker name/routes, Cloudflare account ID, D1 name/ID, endpoint, and owner
identities in external private configuration. Production deployment tooling and
configuration stay in the private repository and are excluded from the export.

The original `.env.pass`, source, and Git history must be preserved outside the
checkout with directories 0700 and files 0600 before extraction. Compare preserved
bytes directly with the original files without printing values. Keep operational
receipts and resource inventories outside Git. This document records a contract,
not machine-specific preservation evidence.

## Private deployment boundary

The existing `.github/workflows/check.yml` and `wrangler.toml` deliberately remain
unchanged in the private repository. They still describe the private runner and
deployment identity. The private pipeline's checked artifact, production health
checks, rollback, migrations gate, and credential isolation retain their behavior.
Local verification explicitly uses `wrangler.local.toml` and synthetic identities.

Do not expose the private workflow to public PRs. Do not substitute the public
workflow on the private default branch: it has no production deployment. The
public candidate is `config/check.public.yml`, an inert file here. In the export
it becomes `.github/workflows/check.yml`: ephemeral hosted Ubuntu PR checks and
full suites only on manual runs and twice daily on the default branch. It has no
push trigger. Only PR runs cancel each other; every full run has its own group.
Checks have read-only contents permission. A separate serialized notification job
has issue-write permission through the built-in job token and opens or updates
one `ci-failure` issue with the failed run URL and SHA. A manual
`notification_drill` input deliberately fails the check before setup to exercise
delivery; its default is false and scheduled runs never enable it. Real delivery
and a green full run on the exact source-review commit remain hosted proof gates.
There is no stored checkout credential, production secret reference, or deploy
step. Native/Worker acceptance uses only temporary local data. Action commits and
toolchain/package versions are pinned.

## Exact extraction

`config/public-export.paths` classifies every committed path. `copy` rows name
each retained source and destination. `exclude` rows name private-only paths;
an absent excluded file is allowed. The exporter refuses any new unclassified
path, duplicate mapping, symlink, submodule, unsafe destination, or existing output
directory. Every Git command disables replace objects. It reads committed blobs
rather than the worktree, prints the resolved commit and tree OIDs, and compares
every written file byte for byte. It never copies `.git`, refs, reflogs, tags, ignored
configuration, build output, evidence, or local exercise records.

After reviewing and committing the candidate, run from its checkout:

```sh
scripts/export-public HEAD /private/scratch/hundred-export
cd /private/scratch/hundred-export
scripts/test-config
cargo test --locked -p hundred-core -p hundred-client
```

The destination's parent must already exist. The export has no default production
Wrangler config. Local checks use `wrangler.local.toml`. For a full proof on an
appropriately budgeted machine, run `scripts/check`, `scripts/test-worker`, and
the native acceptance routes in `docs/verification.md` with synthetic data.
Fresh-export verification must succeed without private config, credentials, a
remote repository, or access to original Git objects. Desktop archive builds
report `source-archive` unless `HUNDRED_BUILD_SHA` is explicitly supplied. Installing
a Git-less archive requires that same nonempty value and an exact binary revision
match. `scripts/test-public-tools` covers this guard, replacement-object exports,
and notification command doubles without accessing a real repository API.

## Publication blockers and cutover

Before publication, independently review the candidate and exported bytes, audit
all retained files for private content and secrets, and verify the generic build
and full suites in isolation. Record the reviewed private source head/tree and
the exported file hashes in external evidence. Audit the history/PR-ref boundary;
never push any original ref or bundle to the public destination.

Use a fresh public identity with a new root commit and no connection to the
private repository's object store. Preserve the private original, ongoing work,
installed application, production resources, and deployment identity. Do not
create that remote as part of this cleanup PR. Audit provenance and author/commit
metadata before the first public commit. The parent owns the final extraction
audit, independent QA/review/judge, repository creation, and publication.

Keep private deployment in its private repository or a separately approved private
delivery pipeline. Any future switch must provision and verify external config
first, test health/rollback with the intended artifact, and explicitly transfer
deployment ownership. Neither this PR nor its hosted export candidate performs
that cutover.
