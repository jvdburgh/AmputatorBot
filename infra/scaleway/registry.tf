# Scaleway Container Registry namespace — holds the Docker images that the
# Serverless Container deploys. Private (the container pulls within the same
# project; public images would be costlier on egress).

resource "scaleway_registry_namespace" "main" {
  name        = local.app_name
  description = "Docker images for the AmputatorBot backend (Rust + Astro static)."
  is_public   = false
}
