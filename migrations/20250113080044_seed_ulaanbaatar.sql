insert into address_area (name, type)
values
  ('"en"=>"Mongolia", "mn"=>"Монгол"', 'Country');

-- Province
with parent_id as (
  select area_id from address_area where name @> '"en"=>"Mongolia"'
)
insert into address_area (name, type, parent_id)
values
  ('"en"=>"Ulaanbaatar", "mn"=>"Улаанбаатар"', 'Province', (select area_id from parent_id));

-- Districts
with parent_id as (
  select area_id
  from address_area
  where name @> '"en"=>"Ulaanbaatar"'
)
insert into address_area (name, type, parent_id)
values
  ('"en"=>"Bagakhangai", "mn"=>"Багахангай"', 'District', (select area_id from parent_id)),
  ('"en"=>"Baganuur", "mn"=>"Багануур"', 'District', (select area_id from parent_id)),
  ('"en"=>"Bayangol", "mn"=>"Баянгол"', 'District', (select area_id from parent_id)),
  ('"en"=>"Bayanzurkh", "mn"=>"Баянзүрх"', 'District', (select area_id from parent_id)),
  ('"en"=>"Chingeltei", "mn"=>"Чингэлтэй"', 'District', (select area_id from parent_id)),
  ('"en"=>"Khan Uul", "mn"=>"Хан Уул"', 'District', (select area_id from parent_id)),
  ('"en"=>"Nalaikh", "mn"=>"Налайх"', 'District', (select area_id from parent_id)),
  ('"en"=>"Songino Khairkhan", "mn"=>"Сонгино хайрхан"', 'District', (select area_id from parent_id)),
  ('"en"=>"Sukhbaatar", "mn"=>"Сүхбаатар"', 'District', (select area_id from parent_id));
