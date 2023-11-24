# Rust AWS Translate CLI Tool

## Overview

This Rust-based command-line tool is designed to translate JSON locale files using AWS Translate to multiple languages.

## Features

- **AWS Profile and Region Configuration**: Customize AWS profile and region settings.
- **Language Support**: Automatically detect and translate to multiple languages.
- **Asynchronous Translation**: Utilize asynchronous tasks for translating to each language concurrently.

## Requirements

- AWS Account and Credentials
- Your Locale file in JSON format (e.g. en.json)

## Usage

1. **Setup AWS Credentials**: Ensure AWS credentials are correctly configured in your environment.
2. **Command-Line Arguments**:
   - `aws_profile`: Specify the AWS profile (default: "default").
   - `aws_region`: Set the AWS region (default: "us-east-1").
   - `source_language_code`: Source language for translation (default: "en").
   - `input_file`: Path to the JSON file to be translated (default: "en.json").
