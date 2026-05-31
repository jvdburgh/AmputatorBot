# Serverless PostgreSQL — scales to zero between requests. Expected steady-state
# bill ~€2-4/mo at the bot's traffic shape (most queries hit the cache; warm
# windows are short).
#
# Scaleway Serverless SQL doesn't use classic Postgres users. Instead the
# container authenticates via an IAM application: the application UUID becomes
# the connection "username" and the IAM API key secret is the "password".
# This block creates that trio (application, scoped policy, key) and assembles
# the final connection string in `local.database_url`, which container.tf
# injects as a *secret* env var on the container.

resource "scaleway_iam_application" "db_client" {
  name        = "${local.app_name}-db-client"
  description = "Identity used by the Scaleway Serverless Container to connect to the Serverless SQL Database."
}

resource "scaleway_iam_policy" "db_client" {
  name           = "${local.app_name}-db-client-readwrite"
  description    = "Read/write access to the Serverless SQL Database within the amputatorbot project."
  application_id = scaleway_iam_application.db_client.id

  rule {
    project_ids          = [data.scaleway_account_project.this.id]
    permission_set_names = ["ServerlessSQLDatabaseReadWrite"]
  }
}

# Anchors the IAM API key's mandatory `expires_at`. Scaleway caps the
# expiration at 1 year from creation (verified at apply time: API returns
# "expiration date ... too far in the future, must be before <now+1y>"),
# so this is the longest period we can pick.
#
# Operational consequence: this credential expires after 1 year. Refresh it
# by running `terraform apply` in the infra/scaleway/ dir at least every
# ~11 months — that re-rotates `time_rotating`, which forces TF to recreate
# `scaleway_iam_api_key.db_client` with a fresh secret, which cascades to
# `local.database_url` and re-injects the new secret into the container.
resource "time_rotating" "db_client_key_expiry" {
  rotation_days = 360
}

resource "scaleway_iam_api_key" "db_client" {
  application_id     = scaleway_iam_application.db_client.id
  description        = "Database connection credential. Used as the password component of DATABASE_URL passed to the container."
  default_project_id = data.scaleway_account_project.this.id
  expires_at         = time_rotating.db_client_key_expiry.rotation_rfc3339
}

resource "scaleway_sdb_sql_database" "main" {
  name    = local.app_name
  min_cpu = 0 # scale to zero when idle — first query pays a wake-up cold start
  max_cpu = 2 # hard ceiling on per-DB compute spend; 1.7M-row cache is small, so 2 vCPU is plenty
}

locals {
  # Format expected by sqlx + libpq:
  #   postgres://<user>:<password>@<host>:<port>/<dbname>?sslmode=require
  # The IAM application UUID acts as the username; the IAM API key secret is
  # the password. Scaleway's exported `endpoint` is already a full URI
  # including the `postgres://` scheme and `?sslmode=require` query string;
  # we strip the scheme and splice in the credentials, leaving sslmode alone.
  database_url = format(
    "postgres://%s:%s@%s",
    scaleway_iam_application.db_client.id,
    scaleway_iam_api_key.db_client.secret_key,
    trimprefix(scaleway_sdb_sql_database.main.endpoint, "postgres://"),
  )
}
