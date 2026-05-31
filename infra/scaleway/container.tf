# Serverless Container — the running production binary. Scales 0..1 instances
# (one-instance ceiling caps worst-case spend; bot scale doesn't need more).

resource "scaleway_container_namespace" "main" {
  name        = local.app_name
  description = "AmputatorBot Serverless Containers."
}

resource "scaleway_container" "backend" {
  name         = "${local.app_name}-backend"
  description  = "Rust API + Astro static site for AmputatorBot."
  namespace_id = scaleway_container_namespace.main.id

  # Image: starts at the :bootstrap tag pushed manually before the first apply.
  # After that, CI's `scw container container update ... image=...:<sha>` swaps
  # it on each release. `lifecycle.ignore_changes` keeps Terraform from
  # reverting CI's tag back to :bootstrap.
  image = "${scaleway_registry_namespace.main.endpoint}/${local.app_name}:${local.bootstrap_image_tag}"
  port  = 8080

  # 140m vCPU / 256 MB — cheapest Scaleway tier. Memory is the more likely
  # pressure point during HTML scrapes; bump memory_limit_bytes to 512_000_000
  # (which forces cpu_limit=280 per the provider's fixed pairing) if Cockpit
  # shows OOMKilled events under real traffic.
  cpu_limit          = 140
  memory_limit_bytes = 256000000

  min_scale = 0
  max_scale = 1

  # Canonical-finding chases redirects + scrapes pages — 30s is generous for
  # the cold path, but the Devvit-side timeout will give up first anyway.
  timeout = 30

  https_connections_only = true

  # PORT is reserved by Scaleway and set automatically from the `port`
  # attribute above — don't list it here or apply rejects with
  # "Reserved environment variable PORT cannot be set".
  environment_variables = {
    RUST_LOG   = "info"
    STATIC_DIR = "/app/static"
  }

  secret_environment_variables = {
    DATABASE_URL = local.database_url
  }

  # Startup probe runs during initial boot. Must succeed before liveness_probe
  # takes over. Generous because cold start = Rust binary load + sqlx migrate
  # check + Serverless PG cold-wake (the DB scales to zero too) — easily 20-30s
  # on a truly cold path.
  startup_probe {
    http {
      path = "/api/v2/health"
    }
    failure_threshold = 30 # 30 * 5s = 150s grace before considering startup failed
    interval          = "5s"
    timeout           = "5s"
  }

  # Liveness runs steady-state after startup succeeds. Restart on persistent
  # unhealthy.
  liveness_probe {
    http {
      path = "/api/v2/health"
    }
    failure_threshold = 3
    interval          = "30s"
    timeout           = "5s"
  }

  lifecycle {
    # CI updates the image tag out-of-band on each release. Without this,
    # `terraform plan` would always want to revert to :bootstrap.
    ignore_changes = [image]
  }
}

# Custom-domain binding for www.amputatorbot.com. Scaleway provisions a
# Let's Encrypt cert for this hostname automatically once the CNAME at
# Cloudflare points at `scaleway_container.backend.public_endpoint`.
# IMPORTANT: during initial issuance the Cloudflare proxy must be OFF
# (grey cloud) so the HTTP-01 challenge reaches the Scaleway endpoint.
# Flip back to orange cloud once the cert is ready.
resource "scaleway_container_domain" "www" {
  container_id = scaleway_container.backend.id
  hostname     = var.custom_hostname
}
