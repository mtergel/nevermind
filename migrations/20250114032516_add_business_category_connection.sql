create table business_category
(
  business_id uuid references business(business_id) on delete cascade,
  category_id uuid references category(category_id) on delete cascade,
  primary key (business_id, category_id)
);
