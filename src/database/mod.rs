#[cfg(test)]
mod tests {
    use sea_orm::Database;

    #[tokio::test]
    async fn test_database() -> eyre::Result<()> {
        let _db = Database::connect("mysql://root:qiuqiu123@localhost").await?;
        Ok(())
    }
}
