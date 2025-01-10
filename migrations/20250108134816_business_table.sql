create extension hstore;

create table business
(
    business_id         uuid primary key default uuid_generate_v1mc(),
    name                hstore not null,
    created_at          timestamptz not null default now(),
    updated_at          timestamptz
);

select trigger_updated_at('business'); 
