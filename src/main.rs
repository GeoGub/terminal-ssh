mod ssh;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ssh::run_ssh_shell().await
}
