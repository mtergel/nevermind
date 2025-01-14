create table category
(
  category_id uuid primary key default uuid_generate_v1mc(),
  name hstore not null,
  parent_id uuid references category(category_id) on delete cascade,
  created_at timestamptz not null default now(),
  updated_at timestamptz
);

select trigger_updated_at('category');
select add_hstore_not_null('category', 'name');
