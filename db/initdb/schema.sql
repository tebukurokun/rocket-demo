create table person (
  id serial primary key,
  name varchar(255) not null,
  age int not null,
  created_at timestamp not null
);

create table career (
  id serial primary key,
  name varchar(255) not null,
  person_id int not null,
  created_at timestamp not null
);
