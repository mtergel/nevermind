create table email
(
    email_id        uuid primary key default uuid_generate_v1mc(),
    user_id         uuid not null references "user" (user_id) on delete cascade,

    email           text collate "case_insensitive" unique not null,
    verified        boolean not null default false,
    is_primary      boolean not null default false, 

    created_at      timestamptz not null default now(),
    updated_at      timestamptz
);

select trigger_updated_at('email'); 
