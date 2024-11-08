create type social_provider AS ENUM ('google', 'facebook', 'github', 'discord');

create table social_login
(
    social_login_id     uuid primary key default uuid_generate_v1mc(),
    email_id            uuid not null references email (email_id) on delete cascade,
    user_id             uuid not null references "user" (user_id) on delete cascade,

    provider            social_provider not null,
    provider_user_id    text not null unique,

    created_at          timestamptz not null default now(),
    updated_at          timestamptz
);

select trigger_updated_at('social_login'); 
