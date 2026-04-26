use sea_orm::Schema;
use sea_orm_migration::prelude::*;

use crate::entities::{
    code_video, interactive_html, knowledge_explanation, knowledge_video, pretest_problem, problem,
    study_quiz, study_quiz_problem, study_stage, study_subject, study_task, user, user_checkin,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(manager.get_database_backend());

        let mut user_table = schema.create_table_from_entity(user::Entity);
        user_table.if_not_exists();
        manager.create_table(user_table).await?;
        manager
            .create_index(
                Index::create()
                    .name("idx-user-username-unique")
                    .table(user::Entity)
                    .col(user::Column::Username)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        let mut user_checkin_table = schema.create_table_from_entity(user_checkin::Entity);
        user_checkin_table.if_not_exists();
        manager.create_table(user_checkin_table).await?;

        let mut knowledge_video_table = schema.create_table_from_entity(knowledge_video::Entity);
        knowledge_video_table.if_not_exists();
        manager.create_table(knowledge_video_table).await?;

        let mut code_video_table = schema.create_table_from_entity(code_video::Entity);
        code_video_table.if_not_exists();
        manager.create_table(code_video_table).await?;

        let mut interactive_html_table = schema.create_table_from_entity(interactive_html::Entity);
        interactive_html_table.if_not_exists();
        manager.create_table(interactive_html_table).await?;

        let mut knowledge_explanation_table =
            schema.create_table_from_entity(knowledge_explanation::Entity);
        knowledge_explanation_table.if_not_exists();
        manager.create_table(knowledge_explanation_table).await?;

        let mut study_subject_table = schema.create_table_from_entity(study_subject::Entity);
        study_subject_table.if_not_exists();
        manager.create_table(study_subject_table).await?;

        let mut problem_table = schema.create_table_from_entity(problem::Entity);
        problem_table.if_not_exists();
        manager.create_table(problem_table).await?;

        let mut pretest_problem_table = schema.create_table_from_entity(pretest_problem::Entity);
        pretest_problem_table.if_not_exists();
        manager.create_table(pretest_problem_table).await?;

        let mut study_stage_table = schema.create_table_from_entity(study_stage::Entity);
        study_stage_table.if_not_exists();
        manager.create_table(study_stage_table).await?;

        let mut study_task_table = schema.create_table_from_entity(study_task::Entity);
        study_task_table.if_not_exists();
        manager.create_table(study_task_table).await?;

        let mut study_quiz_table = schema.create_table_from_entity(study_quiz::Entity);
        study_quiz_table.if_not_exists();
        manager.create_table(study_quiz_table).await?;

        let mut study_quiz_problem_table =
            schema.create_table_from_entity(study_quiz_problem::Entity);
        study_quiz_problem_table.if_not_exists();
        manager.create_table(study_quiz_problem_table).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            "study_quiz_problem",
            "study_quiz",
            "study_task",
            "study_stage",
            "pretest_problem",
            "problem",
            "study_subject",
            "knowledge_explanation",
            "interactive_html",
            "code_video",
            "knowledge_video",
            "user_checkin",
            "user",
        ] {
            manager
                .drop_table(
                    Table::drop()
                        .table(Alias::new(table))
                        .if_exists()
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
