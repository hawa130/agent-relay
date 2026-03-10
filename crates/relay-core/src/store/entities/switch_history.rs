use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "switch_history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub profile_id: Option<String>,
    pub previous_profile_id: Option<String>,
    pub outcome: String,
    pub reason: Option<String>,
    pub checkpoint_id: Option<String>,
    pub rollback_performed: bool,
    pub created_at: String,
    pub details: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
