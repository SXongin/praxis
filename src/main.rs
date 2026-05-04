#[tokio::main]
async fn main() {
    let args = praxis::cli::parse();
    praxis::cli::run(args).await;
}
