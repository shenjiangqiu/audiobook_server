//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "music")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub author_id: i32,
    #[sea_orm(unique)]
    pub name: String,
    pub chapters: i32,
    pub file_folder: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::author::Entity",
        from = "Column::AuthorId",
        to = "super::author::Column::Id",
        on_update = "Restrict",
        on_delete = "Restrict"
    )]
    Author,
    #[sea_orm(has_many = "super::progress::Entity")]
    Progress,
}

impl Related<super::author::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Author.def()
    }
}

impl Related<super::progress::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Progress.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
