variable "region" {
  type        = string
  description = "Scaleway region for every resource in this module. fr-par because Serverless SQL Database is not available in nl-ams (confirmed against scaleway.com/en/product-availability-by-region/, May 2026)."
  default     = "fr-par"
}

variable "app_name" {
  type        = string
  description = "Prefix used for all resource names (DB, registry namespace, container, IAM application)."
  default     = "amputatorbot"
}

variable "project_name" {
  type        = string
  description = "Name of the Scaleway project (looked up via data.scaleway_account_project)."
  default     = "amputatorbot"
}

variable "custom_hostname" {
  type        = string
  description = "Public hostname bound to the container. The Cloudflare DNS CNAME for this name points at the container's public_endpoint; Scaleway issues Let's Encrypt for it automatically once DNS validation succeeds."
  default     = "www.amputatorbot.com"
}
