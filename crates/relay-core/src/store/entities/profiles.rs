use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "profiles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub nickname: String,
    pub agent: String,
    pub priority: i32,
    pub enabled: bool,
    pub account_state: Option<String>,
    pub account_error_http_status: Option<i32>,
    pub account_state_updated_at: Option<String>,
    pub agent_home: Option<String>,
    pub config_path: Option<String>,
    pub auth_mode: String,
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
