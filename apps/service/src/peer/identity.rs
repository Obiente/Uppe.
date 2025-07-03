use anyhow::{Ok, Result};
use iroh::{Endpoint, SecretKey};

#[tokio::main]
pub async fn main() -> Result<()> {
    let secret_key = SecretKey::generate(rand::rngs::OsRng);
    let endpoint = Endpoint::builder()
        .secret_key(secret_key)
        // Enable n0 discovery. This allows you to 
        // dial by `NodeId`, and allows you to be
        // dialed by `NodeId`.
        .discovery_n0()
        .bind()
        .await?;

    println!("> our node id: {}", endpoint.node_id());
    Ok(())
}
