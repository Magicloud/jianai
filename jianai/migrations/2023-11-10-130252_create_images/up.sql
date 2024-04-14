-- Raw big pictures
create table images (
    id serial primary key,
    filename text not null unique,
    digest bytea not null,
    metadata jsonb,
    segmented boolean not null default False
);