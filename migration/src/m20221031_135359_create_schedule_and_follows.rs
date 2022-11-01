use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(entity::scheduled_post::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(entity::scheduled_post::Column::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(entity::scheduled_post::Column::UserId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(entity::scheduled_post::Column::MediaIds)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(entity::scheduled_post::Column::PostText)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(entity::scheduled_post::Column::PostSource)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(entity::scheduled_post::Column::PostSourceUrl)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(entity::follow::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(entity::follow::Column::AuthorId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(entity::follow::Column::UserId)
                            .integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(entity::follow::Column::AuthorId)
                            .col(entity::follow::Column::UserId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(entity::scheduled_post::Entity)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(entity::follow::Entity).to_owned())
            .await
    }
}
