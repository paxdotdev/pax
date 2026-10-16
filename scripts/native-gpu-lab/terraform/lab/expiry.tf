data "archive_file" "expiry" {
  type        = "zip"
  source_file = "${path.module}/../../tools/expire_instances.py"
  output_path = "${path.module}/.terraform/pax-expiry.zip"
}
resource "aws_cloudwatch_log_group" "expiry" {
  name              = "/aws/lambda/${var.lab_name}-expiry"
  retention_in_days = 7
}
resource "aws_iam_role" "expiry" {
  name_prefix = "${var.lab_name}-expiry-"
  assume_role_policy = jsonencode({
    Version   = "2012-10-17"
    Statement = [{ Effect = "Allow", Principal = { Service = "lambda.amazonaws.com" }, Action = "sts:AssumeRole" }]
  })
}
resource "aws_iam_role_policy" "expiry" {
  role = aws_iam_role.expiry.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      { Effect = "Allow", Action = ["ec2:DescribeInstances"], Resource = "*" },
      {
        Effect    = "Allow", Action = ["ec2:StopInstances"]
        Resource  = "arn:aws:ec2:us-west-2:${var.account_id}:instance/*"
        Condition = { StringEquals = { "ec2:ResourceTag/PaxGpuLab" = var.lab_name, "ec2:ResourceTag/ManagedBy" = "pax-gpu-lab" } }
      },
      {
        Effect   = "Allow", Action = ["logs:CreateLogStream", "logs:PutLogEvents"]
        Resource = "${aws_cloudwatch_log_group.expiry.arn}:*"
      }
    ]
  })
}
resource "aws_lambda_function" "expiry" {
  function_name                  = "${var.lab_name}-expiry"
  role                           = aws_iam_role.expiry.arn
  filename                       = data.archive_file.expiry.output_path
  source_code_hash               = data.archive_file.expiry.output_base64sha256
  handler                        = "expire_instances.handler"
  runtime                        = "python3.13"
  timeout                        = 60
  memory_size                    = 128
  reserved_concurrent_executions = 1
  environment {
    variables = { LAB_NAME = var.lab_name }
  }
  depends_on = [aws_iam_role_policy.expiry]
}
resource "aws_scheduler_schedule_group" "expiry" {
  name = "${var.lab_name}-expiry"
}
resource "aws_iam_role" "scheduler" {
  name_prefix = "${var.lab_name}-schedule-"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow", Principal = { Service = "scheduler.amazonaws.com" }, Action = "sts:AssumeRole"
      Condition = {
        StringEquals = { "aws:SourceAccount" = var.account_id }
        ArnEquals    = { "aws:SourceArn" = aws_scheduler_schedule_group.expiry.arn }
      }
    }]
  })
}
resource "aws_iam_role_policy" "scheduler" {
  role = aws_iam_role.scheduler.id
  policy = jsonencode({
    Version   = "2012-10-17"
    Statement = [{ Effect = "Allow", Action = ["lambda:InvokeFunction"], Resource = aws_lambda_function.expiry.arn }]
  })
}
resource "aws_scheduler_schedule" "expiry" {
  name                = "stop-expired-guests"
  group_name          = aws_scheduler_schedule_group.expiry.name
  schedule_expression = "rate(5 minutes)"
  state               = "ENABLED"
  flexible_time_window {
    mode = "OFF"
  }
  target {
    arn      = aws_lambda_function.expiry.arn
    role_arn = aws_iam_role.scheduler.arn
    input    = "{}"
    retry_policy {
      maximum_event_age_in_seconds = 300
      maximum_retry_attempts       = 2
    }
  }
  depends_on = [aws_iam_role_policy.scheduler]
}
