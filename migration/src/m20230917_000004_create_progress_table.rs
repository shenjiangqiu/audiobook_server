// m20220101_000002_create_chef_table.rs

use sea_orm_migration::prelude::*;

use crate::{
    m20230917_000001_create_account_table::Account, m20230917_000003_create_music_table::Music,
};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20230917_000004_create_progress_table" // Make sure this matches with the file name
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration: Create the Chef table.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Progress::Table)
                    .col(
                        ColumnDef::new(Progress::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Progress::AccountId).integer().not_null())
                    .col(ColumnDef::new(Progress::MusicId).integer().not_null())
                    .col(ColumnDef::new(Progress::ChapterNo).integer().not_null())
                    .col(ColumnDef::new(Progress::Progress).double().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-Progress-AccountId")
                            .from(Progress::Table, Progress::AccountId)
                            .to(Account::Table, Account::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-Progress-MusicId")
                            .from(Progress::Table, Progress::MusicId)
                            .to(Music::Table, Music::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    // Define how to rollback this migration: Drop the Progress table.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Progress::Table).to_owned())
            .await
    }
}

// For ease of access
#[derive(Iden)]
pub enum Progress {
    Table,
    Id,
    AccountId,
    MusicId,
    ChapterNo,
    Progress,
}
