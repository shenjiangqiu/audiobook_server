//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "music")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub author: String,
    pub name: String,
    pub chapters: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::progress::Entity")]
    Progress,
}

impl Related<super::progress::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Progress.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
