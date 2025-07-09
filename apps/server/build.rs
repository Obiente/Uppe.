use std::env::vars;

use dotenvy::dotenv;

fn main() {
    dotenv().ok();

    for (k, v,) in vars() {
        println!("cargo:rustc-env={k}={v}");
    }
}
