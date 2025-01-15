alter table "user"
add column fts tsvector generated always as 
  (to_tsvector('simple', username || ' ')) stored;

create index user_fts on "user" using gin (fts);
