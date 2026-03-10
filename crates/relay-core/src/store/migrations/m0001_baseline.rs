use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Profiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Profiles::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Profiles::Nickname).string().not_null())
                    .col(ColumnDef::new(Profiles::Agent).string().not_null())
                    .col(ColumnDef::new(Profiles::Priority).integer().not_null())
                    .col(ColumnDef::new(Profiles::Enabled).boolean().not_null())
                    .col(ColumnDef::new(Profiles::AgentHome).string())
                    .col(ColumnDef::new(Profiles::ConfigPath).string())
                    .col(ColumnDef::new(Profiles::AuthMode).string().not_null())
                    .col(ColumnDef::new(Profiles::Metadata).string().not_null())
                    .col(ColumnDef::new(Profiles::CreatedAt).string().not_null())
                    .col(ColumnDef::new(Profiles::UpdatedAt).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AppSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AppSettings::Key)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AppSettings::Value).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SwitchHistory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SwitchHistory::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SwitchHistory::ProfileId).string())
                    .col(ColumnDef::new(SwitchHistory::PreviousProfileId).string())
                    .col(ColumnDef::new(SwitchHistory::Outcome).string().not_null())
                    .col(ColumnDef::new(SwitchHistory::Reason).string())
                    .col(ColumnDef::new(SwitchHistory::CheckpointId).string())
                    .col(
                        ColumnDef::new(SwitchHistory::RollbackPerformed)
                            .boolean()
                            .not_null(),
                    )
                    .col(ColumnDef::new(SwitchHistory::CreatedAt).string().not_null())
                    .col(ColumnDef::new(SwitchHistory::Details).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(FailureEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FailureEvents::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(FailureEvents::ProfileId).string())
                    .col(ColumnDef::new(FailureEvents::Reason).string().not_null())
                    .col(ColumnDef::new(FailureEvents::Message).string().not_null())
                    .col(ColumnDef::new(FailureEvents::CooldownUntil).string())
                    .col(ColumnDef::new(FailureEvents::CreatedAt).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ProfileProbeIdentities::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProfileProbeIdentities::ProfileId)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ProfileProbeIdentities::Provider)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ProfileProbeIdentities::PrincipalId).string())
                    .col(ColumnDef::new(ProfileProbeIdentities::DisplayName).string())
                    .col(
                        ColumnDef::new(ProfileProbeIdentities::CredentialsJson)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProfileProbeIdentities::MetadataJson)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProfileProbeIdentities::CreatedAt)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ProfileProbeIdentities::UpdatedAt)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AgentSettings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AgentSettings::Agent)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AgentSettings::SettingsJson)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AgentSettings::CreatedAt).string().not_null())
                    .col(ColumnDef::new(AgentSettings::UpdatedAt).string().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AgentSettings::Table).to_owned())
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(ProfileProbeIdentities::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(FailureEvents::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(SwitchHistory::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(AppSettings::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Profiles::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Profiles {
    Table,
    Id,
    Nickname,
    Agent,
    Priority,
    Enabled,
    AgentHome,
    ConfigPath,
    AuthMode,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AppSettings {
    Table,
    Key,
    Value,
}

#[derive(DeriveIden)]
enum SwitchHistory {
    Table,
    Id,
    ProfileId,
    PreviousProfileId,
    Outcome,
    Reason,
    CheckpointId,
    RollbackPerformed,
    CreatedAt,
    Details,
}

#[derive(DeriveIden)]
enum FailureEvents {
    Table,
    Id,
    ProfileId,
    Reason,
    Message,
    CooldownUntil,
    CreatedAt,
}

#[derive(DeriveIden)]
enum ProfileProbeIdentities {
    Table,
    ProfileId,
    Provider,
    PrincipalId,
    DisplayName,
    CredentialsJson,
    MetadataJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AgentSettings {
    Table,
    Agent,
    SettingsJson,
    CreatedAt,
    UpdatedAt,
}
