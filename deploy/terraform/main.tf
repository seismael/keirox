terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "us-east-1"
}

variable "cluster_name" {
  description = "Keirox cluster name"
  type        = string
  default     = "keirox-prod"
}

resource "aws_security_group" "keirox_sg" {
  name        = "${var.cluster_name}-sg"
  description = "Keirox Node Security Group"

  ingress {
    description = "Kafka Gateway"
    from_port   = 9092
    to_port     = 9092
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    description = "Arrow Flight API"
    from_port   = 50051
    to_port     = 50051
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/8"]
  }

  ingress {
    description = "Metrics & Health"
    from_port   = 9090
    to_port     = 9090
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/8"]
  }

  ingress {
    description = "Raft Consensus"
    from_port   = 9091
    to_port     = 9091
    protocol    = "tcp"
    self        = true
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_launch_template" "keirox" {
  name_prefix   = "${var.cluster_name}-"
  image_id      = "ami-0c55b159cbfafe1f0" # Amazon Linux 2023 base
  instance_type = "i3en.2xlarge" # NVMe local instance store

  iam_instance_profile {
    name = aws_iam_instance_profile.keirox_profile.name
  }

  vpc_security_group_ids = [aws_security_group.keirox_sg.id]

  user_data = base64encode(<<-EOF
              #!/bin/bash
              # Setup io_uring and start keirox-server
              mkfs.xfs /dev/nvme1n1
              mount /dev/nvme1n1 /var/lib/keirox/data
              # Run keirox
              docker run -d --net=host -v /var/lib/keirox/data:/var/lib/keirox/data keirox/keirox:1.0.0 start
              EOF
  )
}

resource "aws_autoscaling_group" "keirox_asg" {
  name                = "${var.cluster_name}-asg"
  desired_capacity    = 3
  max_size            = 3
  min_size            = 3
  vpc_zone_identifier = ["subnet-abcdef12", "subnet-abcdef13", "subnet-abcdef14"]

  launch_template {
    id      = aws_launch_template.keirox.id
    version = "$Latest"
  }
}

resource "aws_iam_role" "keirox_role" {
  name = "${var.cluster_name}-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      }
    ]
  })
}

resource "aws_iam_role_policy" "keirox_s3_kms_policy" {
  name = "${var.cluster_name}-policy"
  role = aws_iam_role.keirox_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:PutObject",
          "s3:GetObject",
          "s3:ListBucket"
        ]
        Resource = [
          "arn:aws:s3:::keirox-tier1-storage",
          "arn:aws:s3:::keirox-tier1-storage/*"
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "kms:Encrypt",
          "kms:Decrypt",
          "kms:GenerateDataKey"
        ]
        Resource = "*"
      }
    ]
  })
}

resource "aws_iam_instance_profile" "keirox_profile" {
  name = "${var.cluster_name}-profile"
  role = aws_iam_role.keirox_role.name
}
