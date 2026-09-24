# SUPERSEDED: historical private-origin draft from 2026-09-22; do not apply.
# The selected plan reuses s3://www.pax.dev/blog/ and defers Terraform.
# Read README.md for the current plan and PAX-869 ownership.
terraform {
  required_version = ">= 1.5.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.0"
    }
  }
}

provider "aws" {
  profile = "pax"
  region  = "us-west-2"
}

variable "blog_bucket_name" {
  type        = string
  description = "A new, globally unique bucket name for the blog. Do not supply an existing production bucket."
}

variable "website_certificate_arn" {
  type        = string
  description = "The existing www.pax.dev ACM certificate ARN in us-east-1; obtain from the distribution configuration."
}

# Adoption of the existing distribution, rather than creation of a second one
# claiming www.pax.dev. Establish the team's state backend before applying.
import {
  to = aws_cloudfront_distribution.website
  id = "EYKZZ3KH242XU"
}

resource "aws_s3_bucket" "blog" {
  bucket        = var.blog_bucket_name
  force_destroy = false
  lifecycle {
    prevent_destroy = true
  }
}

resource "aws_s3_bucket_public_access_block" "blog" {
  bucket                  = aws_s3_bucket.blog.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_ownership_controls" "blog" {
  bucket = aws_s3_bucket.blog.id
  rule {
    object_ownership = "BucketOwnerEnforced"
  }
}

resource "aws_s3_bucket_versioning" "blog" {
  bucket = aws_s3_bucket.blog.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_cloudfront_origin_access_control" "blog" {
  name                              = "pax-blog"
  origin_access_control_origin_type = "s3"
  signing_behavior                  = "always"
  signing_protocol                  = "sigv4"
}

resource "aws_s3_bucket_policy" "blog" {
  bucket = aws_s3_bucket.blog.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid       = "CloudFrontReadObjects"
        Effect    = "Allow"
        Principal = { Service = "cloudfront.amazonaws.com" }
        Action    = "s3:GetObject"
        Resource  = "${aws_s3_bucket.blog.arn}/*"
        Condition = { StringEquals = { "AWS:SourceArn" = aws_cloudfront_distribution.website.arn } }
      },
      {
        # S3 returns 404 for absent objects when the requester has ListBucket.
        # This grant is restricted to this distribution, not public bucket access.
        Sid       = "CloudFrontMissingObjectStatus"
        Effect    = "Allow"
        Principal = { Service = "cloudfront.amazonaws.com" }
        Action    = "s3:ListBucket"
        Resource  = aws_s3_bucket.blog.arn
        Condition = { StringEquals = { "AWS:SourceArn" = aws_cloudfront_distribution.website.arn } }
      }
    ]
  })
}

resource "aws_cloudfront_cache_policy" "blog" {
  name        = "PaxBlog"
  min_ttl     = 0
  default_ttl = 60
  max_ttl     = 31536000
  parameters_in_cache_key_and_forwarded_to_origin {
    enable_accept_encoding_brotli = true
    enable_accept_encoding_gzip   = true
    cookies_config {
      cookie_behavior = "none"
    }
    headers_config {
      header_behavior = "none"
    }
    # The pinned theme fingerprints stylesheet URLs using ?h=… .
    query_strings_config {
      query_string_behavior = "whitelist"
      query_strings {
        items = ["h"]
      }
    }
  }
}

resource "aws_cloudfront_function" "blog" {
  name    = "pax-blog-paths"
  runtime = "cloudfront-js-2.0"
  comment = "Resolve /blog documents and assets in a separate static origin"
  publish = true
  code    = file("${path.module}/blog-paths.js")
}

resource "aws_cloudfront_distribution" "website" {
  enabled             = true
  is_ipv6_enabled     = true
  aliases             = ["www.pax.dev"]
  default_root_object = ""
  price_class         = "PriceClass_All"
  http_version        = "http2"
  comment             = ""
  retain_on_delete    = true
  wait_for_deployment = true

  # Preserve the existing homepage origin and its historical ID.
  origin {
    origin_id   = "pax-designer.webflow.io"
    domain_name = "www.pax.dev.s3-website-us-west-2.amazonaws.com"
    custom_origin_config {
      http_port                = 80
      https_port               = 443
      origin_protocol_policy   = "http-only"
      origin_ssl_protocols     = ["SSLv3", "TLSv1", "TLSv1.1", "TLSv1.2"]
      origin_read_timeout      = 30
      origin_keepalive_timeout = 5
    }
  }

  origin {
    origin_id                = "pax-blog"
    domain_name              = aws_s3_bucket.blog.bucket_regional_domain_name
    origin_access_control_id = aws_cloudfront_origin_access_control.blog.id
  }

  default_cache_behavior {
    target_origin_id       = "pax-designer.webflow.io"
    allowed_methods        = ["GET", "HEAD"]
    cached_methods         = ["GET", "HEAD"]
    viewer_protocol_policy = "redirect-to-https"
    compress               = true
    cache_policy_id        = "658327ea-f89d-4fab-a63d-7e88639e58f6"
  }

  dynamic "ordered_cache_behavior" {
    for_each = ["/blog", "/blog/*"]
    content {
      path_pattern           = ordered_cache_behavior.value
      target_origin_id       = "pax-blog"
      allowed_methods        = ["GET", "HEAD"]
      cached_methods         = ["GET", "HEAD"]
      viewer_protocol_policy = "redirect-to-https"
      compress               = true
      cache_policy_id        = aws_cloudfront_cache_policy.blog.id
      function_association {
        event_type   = "viewer-request"
        function_arn = aws_cloudfront_function.blog.arn
      }
    }
  }

  # No distribution-wide SPA error fallback: blog misses must remain failures.
  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }
  viewer_certificate {
    acm_certificate_arn      = var.website_certificate_arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }
  lifecycle {
    prevent_destroy = true
  }
}

output "blog_bucket" {
  value = aws_s3_bucket.blog.id
}
