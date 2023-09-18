#[cfg(test)]
mod tests {
    use sea_orm::Database;

    #[tokio::test]
    async fn test_database() -> eyre::Result<()> {
        let db = Database::connect("mysql://root:qiuqiu123@10.10.0.2").await?;
        Ok(())
    }
}
