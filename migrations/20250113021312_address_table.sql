create extension postgis;

create type addr_area_type as enum ('Country', 'Province', 'City', 'District');
-- create type addr_el_type as enum ('StreetName', 'BuildingNumber', 'PostalCode', 'Unit', 'Floor');

create table address_area
(
  area_id uuid primary key default uuid_generate_v1mc(),
  name hstore not null,
  type addr_area_type not null,
  parent_id uuid references address_area(area_id) on delete cascade,

  created_at  timestamptz not null default now(),
  updated_at  timestamptz

);

select trigger_updated_at('address_area');
select add_hstore_not_null('address_area', 'name'); 

-- Country: Mongolia
-- Province: Tuv
-- City: Ulaanbaatar
-- District: Bagakhangai, Baganuur, Bayangol etc...
