pub mod common;

use common::helpers::spawn_app;
use serde::{Deserialize, Serialize};
use sqlx::postgres::types::PgHstore;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Business {
    pub business_id: Uuid,
    pub name: String,
}

// TODO: Move away from admin list business table
// to remove extra setup code

#[tokio::test]
async fn list_localization_works() {
    let app = spawn_app().await;

    // adding permission
    sqlx::query!(
        r#"
            insert into user_role (user_id, role)
            values ($1, $2::app_role)
        "#,
        app.test_user.user_id,
        "root" as _
    )
    .execute(&app.db_pool)
    .await
    .expect("failed to add permission");

    let token = app.login_and_get_token().await;

    // add business
    let name_data = PgHstore::from_iter([
        ("en".to_string(), Some("Sakura Café".to_string())),
        ("mn".to_string(), Some("Сакура Кафе".to_string())),
    ]);

    let id = sqlx::query_scalar!(
        r#"
            insert into business (name)
            values ($1)
            returning business_id
        "#,
        name_data as _
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("failed to add business");

    // fetch no localization should default to en
    let res = app
        .api_client
        .get(&format!("{}/admin/business/{}", &app.address, &id))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    // assert data
    let res = res.json::<Business>().await.unwrap();
    assert_eq!(res.name, "Sakura Café");
}
