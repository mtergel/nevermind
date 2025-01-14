pub mod common;

use common::helpers::spawn_app;
use nevermind::app::utils::types::Timestamptz;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::{Date, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct FakeUser {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: Timestamptz,
}

// TODO: Move away from admin list user table
// to remove extra setup code

#[tokio::test]
async fn list_pagination_works() {
    let app = spawn_app().await;
    app.add_permission("user.read").await;

    let token = app.login_and_get_token().await;

    // seed data
    let items = generate_fake_users(60, &app.db_pool).await;
    let page_size = 25;

    let chunks = items.chunks(page_size);
    let mut next_cursor: Option<String> = None;

    for chunk in chunks {
        // assert request
        let res = app
            .api_client
            .get(&format!("{}/admin/users", &app.address))
            .header("Authorization", "Bearer ".to_owned() + &token)
            .header("Content-Type", "application/json")
            .query(&[("cursor", next_cursor.clone())])
            .send()
            .await
            .expect("failed to execute request");
        assert!(res.status().is_success());

        // assert data
        let res = res.json::<UserListResponse>().await.unwrap();
        let chunk_size = if chunk.len() != page_size {
            chunk.len() + 1 // test user from spawn_app
        } else {
            chunk.len()
        };

        assert_eq!(res.data.len(), chunk_size);

        for (x, y) in chunk.iter().zip(res.data.iter()) {
            assert_eq!(x.username, y.username);
            assert_eq!(x.user_id, y.user_id);
            assert_eq!(x.created_at.0, y.created_at.0);
        }

        if chunk.len() == page_size {
            assert!(res.next_cursor.is_some())
        } else {
            assert!(res.next_cursor.is_none())
        }

        next_cursor = res.next_cursor
    }
}

// Other test helper codes
#[derive(Deserialize)]
pub struct UserListResponse {
    data: Vec<UserData>,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserData {
    user_id: Uuid,
    username: String,
    created_at: Timestamptz,
}

async fn generate_fake_users(capacity: usize, db_pool: &PgPool) -> Vec<FakeUser> {
    let mut items: Vec<FakeUser> = Vec::with_capacity(capacity);

    // Insert into database
    for _ in 0..capacity {
        let user = FakeUser::generate();
        user.store(&db_pool).await;
        items.push(user)
    }

    // Sort by created_at, id
    // for evaluation
    items.sort_by(|a, b| {
        b.created_at
            .0
            .cmp(&a.created_at.0)
            .then(b.user_id.cmp(&a.user_id))
    });

    items
}

impl FakeUser {
    pub fn generate() -> Self {
        FakeUser {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            email: Uuid::new_v4().to_string(),
            created_at: Timestamptz(random_offset_datetime()),
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

fn random_offset_datetime() -> OffsetDateTime {
    let mut rng = rand::thread_rng();

    // Generate a random year, month, and day
    let current_year = OffsetDateTime::now_utc().year();

    // Doind this cause, the test_user is being created at now
    let start_year = current_year + 1; // Next year
    let end_year = start_year + 20; // 20 years after next year

    let year = rng.gen_range(start_year..=end_year); // Adjust range as needed
    let month = rng.gen_range(1..=12);
    let day = rng.gen_range(1..=28); // Simplified to avoid invalid dates (e.g., February 30)

    // Generate a random hour, minute, second, and nanosecond
    let hour = rng.gen_range(0..24);
    let minute = rng.gen_range(0..60);
    let second = rng.gen_range(0..60);
    let nanosecond = rng.gen_range(0..1_000_000_000);

    // Build the OffsetDateTime
    let date = Date::from_calendar_date(year, month.try_into().unwrap(), day).unwrap();
    let time = Time::from_hms_nano(hour, minute, second, nanosecond).unwrap();
    let naive_datetime = PrimitiveDateTime::new(date, time);

    // postgres
    // Convert to UTC and adjust to microsecond precision
    let utc_datetime = naive_datetime.assume_offset(UtcOffset::UTC);
    to_microsecond_precision(utc_datetime)
}

fn to_microsecond_precision(datetime: OffsetDateTime) -> OffsetDateTime {
    let microseconds = datetime.unix_timestamp_nanos() / 1_000; // Convert nanoseconds to microseconds
    OffsetDateTime::from_unix_timestamp_nanos(microseconds * 1_000).unwrap() // Convert back to nanoseconds
}
