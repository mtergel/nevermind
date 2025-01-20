pub mod common;

use common::helpers::spawn_app;
use nevermind::app::utils::types::Timestamptz;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct FakeUser {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: Timestamptz,
}

#[tokio::test]
async fn list_filter_works() {
    let app = spawn_app().await;
    app.add_role("root").await;

    let token = app.login_and_get_token().await;

    let user_data = vec![
        ("car00ll", "ca_rol32@mail.com"),
        ("alice", "alice@mail.com"),
        ("alice_liddel", "alice_liddel19@mail.com"),
        ("aliceaqwedf", "alice1024@mail.com"),
        ("bob_myers", "bob_gs@mail.com"),
        ("bob_alice", "alice_real_bob@mail.com"),
    ];

    let mut items = Vec::with_capacity(user_data.len());

    for (username, email) in user_data {
        let user = FakeUser::generate(username, email);
        user.store(&app.db_pool).await;
        items.push(user);
    }

    // assert request
    let res = app
        .api_client
        .get(&format!("{}/admin/users", &app.address))
        .header("Authorization", "Bearer ".to_owned() + &token)
        .header("Content-Type", "application/json")
        .query(&[("term", "alice:*")])
        .send()
        .await
        .expect("failed to execute request");

    assert!(res.status().is_success());

    // assert data
    let res = res.json::<UserListResponse>().await.unwrap();
    for x in res.data.iter() {
        assert!(x.username.starts_with("alice"));
    }
}

// Other test helper codes
#[derive(Deserialize)]
pub struct UserListResponse {
    data: Vec<UserData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserData {
    user_id: Uuid,
    username: String,
    verified: bool,
    created_at: Timestamptz,
}

impl FakeUser {
    pub fn generate(username: &str, email: &str) -> Self {
        FakeUser {
            user_id: Uuid::new_v4(),
            username: username.to_string(),
            email: email.to_string(),
            created_at: Timestamptz(OffsetDateTime::now_utc()),
        }
    }

    async fn store(&self, pool: &PgPool) {
        sqlx::query!(
            r#"
                insert into "user" (user_id, username, password_hash, created_at)
                values ($1, $2, $3, $4)
            "#,
            self.user_id,
            self.username,
            "FAKE_HASH",
            self.created_at.0
        )
        .execute(pool)
        .await
        .expect("failed to store test user");

        sqlx::query!(
            r#"
                insert into email (user_id, email, verified, is_primary, created_at)
                values ($1, $2, $3, $4, $5)
            "#,
            self.user_id,
            self.email,
            true,
            true,
            self.created_at.0
        )
        .execute(pool)
        .await
        .expect("failed to store test user");
    }
}
