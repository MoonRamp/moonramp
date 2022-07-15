use moonramp_core::sea_orm;
use moonramp_entity::role::*;
use sea_orm::Schema;
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220330_000003_create_roles_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let builder = manager.get_database_backend();
        let schema = Schema::new(builder);
        let create_table = schema.create_table_from_entity(Entity);
        manager.create_table(create_table).await?;
        let create_indexs = schema.create_index_from_entity(Entity);
        for create_index in create_indexs {
            manager.create_index(create_index).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Entity).to_owned())
            .await
    }
}
