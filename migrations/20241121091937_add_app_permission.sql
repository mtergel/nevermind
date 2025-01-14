create type app_permission as enum ('user.create', 'user.read', 'user.update','user.delete');

create table user_permission
(
    user_id         uuid not null references "user" (user_id) on delete cascade,
    permission      app_permission not null,
    added_by        uuid references "user" (user_id) on delete set null,
    created_at      timestamptz not null default now(),
    primary key (user_id, permission)
);
