use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "profile_probe_identities")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub profile_id: String,
    pub provider: String,
    pub principal_id: Option<String>,
    pub display_name: Option<String>,
    pub credentials_json: String,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
