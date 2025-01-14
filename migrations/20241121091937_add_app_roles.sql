-- This role is for application wide
create type app_role as enum ('root', 'moderator');
create type app_permission as enum ('user.view');

create table user_role
(
    user_role_id    uuid primary key default uuid_generate_v1mc(),
    user_id         uuid not null references "user" (user_id) on delete cascade,
    role            app_role not null,
    unique          (user_id, role)
);


create table role_permission
(
    role_permission_id      uuid primary key default uuid_generate_v1mc(),
    role                    app_role not null,
    permission              app_permission not null,
    unique                  (role, permission)
);


-- seed initial
insert into role_permission (role, permission)
values
    ('root', 'user.view'),
    ('moderator', 'user.view');
