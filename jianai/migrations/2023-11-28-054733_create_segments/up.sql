-- Cats
-- if there was tag, then the identifying was wrong. Use this for training unless low_quality.
create table segments (
    id serial primary key,
    image_id integer not null references images(id),
    bounding_box box not null,
    identified_as integer references tags(id),
    tagged_as integer references tags(id),
    low_quality boolean not null default False
);