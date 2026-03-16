use std::sync::Arc;

use futures::StreamExt;

use brain_providers::openai_oauth::preset::OpenAiOAuthPreset;
use brain_providers::OpenAiOAuthProvider;
use brain_stores::FileStore;
use brain_types::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let store_dir = std::env::current_dir()?.join(".agents");
    let store = Arc::new(FileStore::new(&store_dir).await?)
        as Arc<dyn CredentialStore>;

    let preset = &OpenAiOAuthPreset::OPENAI;

    // Try loading stored credentials first
    if let Some(provider) = OpenAiOAuthProvider::from_stored(store.clone(), preset).await? {
        println!("Found stored OAuth credentials, testing chat...");
        test_chat(&provider).await?;
        return Ok(());
    }

    // No stored credentials — pick a flow
    let arg = std::env::args().nth(1).unwrap_or_default();
    let provider = if arg == "--device" {
        println!("Starting device code flow...\n");
        OpenAiOAuthProvider::login_device(store, preset, |prompt| {
            println!("Go to:     {}", prompt.verification_url);
            println!("Enter code: {}", prompt.user_code);
            println!("\nWaiting for authorization...");
        }).await?
    } else {
        println!("Starting browser login flow...\n");
        OpenAiOAuthProvider::login_browser(store, preset, |prompt| {
            println!("Go to: {}\n", prompt.url);
            println!("Complete authorization in your browser.");
            println!("Waiting for callback on port {}...", preset.callback_port);
        }).await?
    };

    println!("\nLogin successful! Testing chat...\n");
    test_chat(&provider).await?;

    Ok(())
}

async fn test_chat(provider: &impl Provider) -> anyhow::Result<()> {
    let messages = vec![Message::user("Say hello in exactly 5 words.")];
    let config = InferenceConfig {
        model: None,
        ..Default::default()
    };

    let mut stream = provider.chat(&messages, &[], &config, None).await?;
    print!("Response: ");
    while let Some(chunk) = stream.next().await {
        match chunk? {
            ChatChunk::Delta { content } => print!("{content}"),
            ChatChunk::Done { usage } => {
                println!();
                if let Some(u) = usage {
                    println!("Tokens: {} prompt + {} completion = {} total",
                        u.prompt, u.completion, u.total);
                }
            }
            ChatChunk::ToolCall { name, .. } => println!("[tool call: {name}]"),
            _ => {}
        }
    }
    Ok(())
}
