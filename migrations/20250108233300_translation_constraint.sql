create extension if not exists hstore;

create or replace function add_hstore_not_null(
    tablename regclass,
    columnname text
) returns void as $$
declare
    constraint_name text;
begin
    constraint_name := 'chk_l10n_' || tablename || '_' || columnname || '_en_not_null';
    execute format('ALTER TABLE %I
            ADD CONSTRAINT %I
            CHECK (%I ? ''en'' AND %I->''en'' IS NOT NULL);',
        tablename, constraint_name, columnname, columnname);
end;
$$ language plpgsql;
