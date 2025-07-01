use anyhow::Result;
use iroh::{Endpoint, SecretKey};

#[tokio::main]
async fn main() -> Result<()> {
    let secret_key: SecretKey = SecretKey::generate(rand::rngs::OsRng);
    println!("secret_key {secret_key}");
}
