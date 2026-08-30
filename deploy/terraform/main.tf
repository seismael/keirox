# Terraform module for Keirox 3-Node High-Availability Cluster on AWS per KEI-K8S-501

terraform {
  required_version = ">= 1.5.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

variable "environment" {
  type        = string
  default     = "production"
  description = "Target deployment environment"
}

variable "node_count" {
  type        = number
  default     = 3
  description = "Number of Raft quorum nodes in Keirox cluster"
}

resource "aws_s3_bucket" "lakehouse" {
  bucket = "keirox-${var.environment}-lakehouse-data"
}

resource "aws_kms_key" "keirox_master" {
  description             = "Keirox Master Envelope Key Encryption Key (KEK)"
  deletion_window_in_days = 30
  enable_key_rotation     = true
}

output "lakehouse_bucket" {
  value = aws_s3_bucket.lakehouse.id
}

output "kms_key_arn" {
  value = aws_kms_key.keirox_master.arn
}
