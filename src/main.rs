use anyhow::Result;
use async_recursion::async_recursion;
use aws_config::Region;
use aws_sdk_translate as translate;
use clap::Parser;
use env_logger::Env;
use log::{debug, info};
use serde_json::{to_string_pretty, Map, Value};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    time::Duration,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// AWS Profile to use
    #[arg(long, default_value = "default")]
    aws_profile: String,

    /// AWS Region to use
    #[arg(long, default_value = "us-east-1")]
    aws_region: String,

    /// Input file to translate
    #[arg(long, default_value = "assets/original/en.json")]
    input_file: String,

    /// Source language code
    #[arg(long, default_value = "en")]
    source_language_code: String,
}

#[::tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let env = Env::default()
        .filter_or("RUST_LOG", "info");

    env_logger::init_from_env(env);

    // start timer
    let start = std::time::Instant::now();
    info!("Starting translation");

    let config = aws_config::from_env()
        .credentials_provider(
            aws_config::profile::ProfileFileCredentialsProvider::builder()
                .profile_name(args.aws_profile)
                .build(),
        )
        .region(Region::new(args.aws_region))
        .load()
        .await;

    let client = translate::Client::new(&config);

    let language_codes = match client.list_languages().send().await {
        Ok(resp) => resp.languages.unwrap_or_default(),
        Err(err) => return Err(err.into()), // Error is now properly handled
    };

    let mut handles = Vec::new();

    for language_code in &language_codes {
        let source_language_code = args.source_language_code.clone();

        // Skip the auto language
        if language_code.language_code == "auto" {
            debug!("Skipping auto language");
            continue;
        }

        // Skip the source language
        if language_code.language_code == source_language_code {
            debug!("Skipping source language");
            continue;
        }

        // Clone the variables to move into the asynchronous task
        let language_code = language_code.clone(); // Clone the language code
        let input_file = args.input_file.clone();
        let client = client.clone();

        // Spawn a new asynchronous task for each language translation
        let original_language_code = args.source_language_code.clone();
        let mut original_file_content = File::open(input_file.clone())?;
        let target_language_code = language_code;

        let handle = tokio::spawn(async move {
            create_translation_file(original_language_code.as_str(), target_language_code.language_code(), &mut original_file_content, client).await
        });

        // Store the task handle
        handles.push(handle);
    }

    // Wait for all the spawned tasks to complete
    for handle in handles {
        handle.await??;
    }

    // end timer
    let duration = start.elapsed();
    info!("Time elapsed: {:?}", duration);
    // remove one for auto, one for source language
    info!("Completed {} translations", language_codes.len() - 2);
    Ok(())
}

async fn create_translation_file(
    original_language_code: &str,
    target_language_code: &str,
    original_file_content: &mut File,
    translate_client: aws_sdk_translate::Client,
) -> Result<()> {
    let mut original_content = String::new();
    original_file_content.read_to_string(&mut original_content)?;

    // Parse the JSON content
    let json_value: Value = serde_json::from_str(&original_content)?;

    // Implement retry logic
    let mut retries = 0;
    let max_retries = 5;
    let mut delay = Duration::from_secs(1); // Starting delay of 1 second

    // Recursively translate the JSON object
    let translated_json = loop {
        match translate_json_object(original_language_code, target_language_code, json_value.clone(), &translate_client).await {
            Ok(translated_json) => {
                break to_string_pretty(&translated_json)?
            },
            Err(_) if retries < max_retries => {
                tokio::time::sleep(delay).await;
                retries += 1;
                delay *= 2; // Exponential backoff
                continue;
            }
            Err(err) => return Err(err.into()),
        }
    };

    // Write the translated JSON to a new file
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(format!("assets/translated/{}.json", target_language_code))?;

    file.write_all(translated_json.as_bytes())?;

    Ok(())
}

#[async_recursion]
async fn translate_json_object(
    source_language_code: &str,
    target_language: &str,
    json_value: Value,
    translate_client: &aws_sdk_translate::Client,
) -> Result<Value, translate::Error> {
    match json_value {
        Value::Object(obj) => {
            let mut new_obj = Map::new();
            for (k, v) in obj {
                match translate_json_object(
                    source_language_code,
                    target_language,
                    v,
                    translate_client,
                )
                .await
                {
                    Ok(translated_value) => {
                        new_obj.insert(k, translated_value);
                    }
                    Err(err) => return Err(err),
                }
            }
            Ok(Value::Object(new_obj))
        }
        Value::String(s) => {
            if s == "" { return Ok(Value::String("".to_string())); }
            let translated_text = translate_client
                .translate_text()
                .source_language_code(source_language_code)
                .target_language_code(target_language)
                .text(&s)
                .send()
                .await?
                .translated_text;

            Ok(Value::String(translated_text))
        }
        _ => Ok(json_value), // Non-string values are left as-is
    }
}
