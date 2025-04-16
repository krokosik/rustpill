use std::io::{Write, stdout};

use host::{RustpillClient, read_line};

#[tokio::main]
async fn main() {
    println!("Connecting to USB device...");
    let client = RustpillClient::new();
    println!("Connected! Pinging 42");
    let ping = client.pingx2(42).await.unwrap();
    println!("Got: {ping}.");
    let uid = client.get_id().await.unwrap();
    println!("ID: {uid:024X}");
    println!();

    // Begin repl...
    loop {
        print!("> ");
        stdout().flush().unwrap();
        let line = read_line().await;
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
            ["ping"] => {
                let ping = client.pingx2(42).await.unwrap();
                println!("Got: {ping}.");
            }
            ["ping", n] => {
                let Ok(idx) = n.parse::<u32>() else {
                    println!("Bad u32: '{n}'");
                    continue;
                };
                let ping = client.pingx2(idx).await.unwrap();
                println!("Got: {ping}.");
            }
            ["angle", n] => {
                let Ok(angle) = n.parse::<u8>() else {
                    println!("Bad u8: '{n}'");
                    continue;
                };
                client.set_angle(angle).await.unwrap();
            }
            ["angle"] => {
                let angle = client.get_angle().await.unwrap();
                println!("Got: {angle}.");
            }
            ["id"] => {
                let uid = client.get_id().await.unwrap();
                println!("ID: {uid:024X}");
            }
            ["exit"] | ["quit"] => {
                break;
            }
            ["schema"] => {
                let schema = client.client.get_schema_report().await.unwrap();

                println!();
                println!("# Endpoints");
                println!();
                for ep in &schema.endpoints {
                    println!("* '{}'", ep.path);
                    println!("  * Request:  {}", ep.req_ty);
                    println!("  * Response: {}", ep.resp_ty);
                }

                println!();
                println!("# Topics Client -> Server");
                println!();
                for tp in &schema.topics_in {
                    println!("* '{}'", tp.path);
                    println!("  * Message: {}", tp.ty);
                }

                println!();
                println!("# Topics Client <- Server");
                println!();
                for tp in &schema.topics_out {
                    println!("* '{}'", tp.path);
                    println!("  * Message: {}", tp.ty);
                }
                println!();
            }
            other => {
                println!("Error, didn't understand '{other:?};");
            }
        }
    }
}
