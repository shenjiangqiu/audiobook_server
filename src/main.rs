use audiobook_server::app_main;
#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    app_main().await?;
    Ok(())
}
