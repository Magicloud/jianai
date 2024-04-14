-- cat names
create table tags (
    id serial primary key,
    tag text not null unique
);