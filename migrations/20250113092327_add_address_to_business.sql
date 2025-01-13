alter table address
add column business_id uuid references business(business_id) on delete cascade;
