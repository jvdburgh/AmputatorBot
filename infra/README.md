# Infrastructure

Terraform for the production deploy on Scaleway. See the per-milestone plan at
`.claude/skills/amputatorbot-migration/SKILL.md` for the full M6 sequence.

## Layout

- `scaleway/` — one module managing the Serverless PostgreSQL database (with the
  IAM application + policy + API key that the container uses to connect),
  the Container Registry namespace, the Serverless Container itself, and the
  custom-domain binding for `www.amputatorbot.com`.

Cloudflare changes (DNS swap, rate-limit rule) are not in Terraform — those are
two clicks in the Cloudflare dashboard and the runbook lives in the M6 plan.

## Prerequisites

- `terraform` ≥ 1.6 on PATH (`brew install terraform`).
- `scw` configured locally (`scw init`) with `default_region = fr-par` and
  `default_project_id` pointing at the `amputatorbot` project. The Scaleway
  Terraform provider reads `~/.config/scw/config.yaml` automatically.

## First apply (chicken-and-egg)

The container resource references a `:bootstrap` image tag that doesn't exist
yet, so the first apply has to come in two passes:

```bash
cd infra/scaleway
terraform init

# Pass 1 — foundation: DB, IAM, registry, container namespace (no container yet).
terraform apply \
  -target=scaleway_iam_application.db_client \
  -target=scaleway_iam_policy.db_client \
  -target=scaleway_iam_api_key.db_client \
  -target=scaleway_sdb_sql_database.main \
  -target=scaleway_registry_namespace.main \
  -target=scaleway_container_namespace.main

# Step 4 of M6: build + push the bootstrap image to the registry that just got
# created. See `terraform output registry_endpoint` for the URL.

# Pass 2 — full apply, now succeeds: container + custom domain.
terraform apply
```

After this first run, only `terraform apply` is needed for infra changes. The
container's `image` is set in `lifecycle.ignore_changes`, so subsequent CI
deploys updating the live image won't show as TF drift.

## State file

Local state, gitignored, contains the DB API key secret. Back up
`terraform.tfstate` to Bitwarden after every successful apply (Bitwarden secure
note → "Scaleway TF state — amputatorbot" → attachment). Losing it isn't
catastrophic but rebuilding from scratch would rotate the DB credentials.

## Useful outputs

```bash
terraform output -raw registry_endpoint        # rg.fr-par.scw.cloud/amputatorbot
terraform output -raw container_endpoint       # https://<...>.functions.fnc.fr-par.scw.cloud
terraform output -raw container_id             # used as SCW_CONTAINER_ID in GitHub Actions
terraform output -raw database_url             # sensitive; for psql data migration
```
