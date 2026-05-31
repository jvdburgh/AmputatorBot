output "registry_endpoint" {
  description = "Docker registry endpoint, e.g. rg.nl-ams.scw.cloud/amputatorbot. Used as the prefix when pushing images."
  value       = scaleway_registry_namespace.main.endpoint
}

output "container_id" {
  description = "Container ID in the format <region>/<uuid>. Goes into the SCW_CONTAINER_ID GitHub Actions secret so release.yml can call `scw container container update`."
  value       = scaleway_container.backend.id
}

output "container_endpoint" {
  description = "Scaleway-issued public hostname for the container (https://<...>.functions.fnc.nl-ams.scw.cloud). Used for direct smoke tests before DNS swap, and as the CNAME target at Cloudflare."
  value       = scaleway_container.backend.public_endpoint
}

output "custom_domain_status" {
  description = "The scaleway_container_domain resource ID. Cert status visible in the Scaleway console; this output just confirms TF knows about the binding."
  value       = scaleway_container_domain.www.id
}

output "database_url" {
  description = "Full DATABASE_URL for psql / sqlx. Already includes IAM application UUID as user, IAM api key secret as password, and sslmode=require. Used by Step 9 of the M6 runbook for the \\copy data migration."
  value       = local.database_url
  sensitive   = true
}
