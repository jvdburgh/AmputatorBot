terraform {
  required_version = ">= 1.6"

  required_providers {
    scaleway = {
      source  = "scaleway/scaleway"
      version = "~> 2.75"
    }
    # Used to anchor the IAM API key's `expires_at`. The Scaleway API rejects
    # keys with no expiration date even though the provider docs mark it as
    # optional; `time_rotating` lets us set a fixed-future date that
    # Terraform can also rotate later by bumping the rotation argument.
    time = {
      source  = "hashicorp/time"
      version = "~> 0.12"
    }
  }
}
