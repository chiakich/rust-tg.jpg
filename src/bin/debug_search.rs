// Debug binary to test Google Image Search
// Run with: RUST_LOG=debug cargo run --bin debug_search

use tgjpg_rs::google_image_searcher;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    
    println!("=== Testing Google Image Search ===");
    println!("Query: cat, is_gif: false");
    println!();
    
    match google_image_searcher::search("cat", false).await {
        Ok(urls) => {
            println!("SUCCESS: Got {} URLs", urls.len());
            for (i, url) in urls.iter().enumerate() {
                println!("  [{}] {}", i + 1, url);
            }
        }
        Err(e) => {
            println!("FAILED: {:?}", e);
        }
    }
}
