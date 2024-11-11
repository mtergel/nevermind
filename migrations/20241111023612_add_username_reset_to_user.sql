alter table "user"
-- if true user must reset their username
-- might be set from admin or social register
add column reset_username boolean;
