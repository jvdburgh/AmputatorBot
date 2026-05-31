# Provider auto-loads credentials + default_project_id from ~/.config/scw/config.yaml.
# Override with SCW_ACCESS_KEY / SCW_SECRET_KEY / SCW_DEFAULT_PROJECT_ID env vars
# when running outside the dev machine (CI doesn't run terraform — only the
# bash-side `scw container container update`).
provider "scaleway" {
  region = var.region
}

# Project lookup. The IAM policy + api key resources need the project UUID;
# pulling it via data source keeps the UUID out of the TF code.
data "scaleway_account_project" "this" {
  name = var.project_name
}

locals {
  app_name = var.app_name

  # Placeholder tag for the first deploy. CI subsequently updates the live
  # container's image to git-SHA tags via `scw container container update`;
  # the container resource has `lifecycle.ignore_changes = [image]` so that
  # CI-driven bumps don't show as Terraform drift.
  bootstrap_image_tag = "bootstrap"
}
