-- Your SQL goes here
create table uuid_mapping (
  mapped_id bigint unsigned primary key not null auto_increment,
  uuid char(36) NOT NULL,
  index uuid (uuid)
);

-- #region break counts
create table break_count_snapshot_point (
  id bigint unsigned primary key not null auto_increment,
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp)
);

create table break_count_snapshot (
  snapshot_point_id bigint unsigned not null references break_count_snapshot_point(id),
  mapped_player_id bigint unsigned not null references uuid_mapping(mapped_id),
  value bigint unsigned not null,
  primary key (snapshot_point_id, mapped_player_id),
  index mapped_player_id (mapped_player_id)
);

create table break_count_diff_point (
  id bigint unsigned primary key not null auto_increment,
  root_snapshot_point_id bigint unsigned not null references break_count_snapshot_point(id),
  previous_diff_point_id bigint unsigned null references break_count_diff_point(id),
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp),
  index root_id_then_timestamp (root_snapshot_point_id, record_timestamp)
);

create table break_count_diff (
  diff_point_id bigint unsigned not null references break_count_diff_point(id),
  mapped_player_id bigint unsigned not null references uuid_mapping(mapped_id),
  new_value bigint unsigned not null,
  primary key (diff_point_id, mapped_player_id),
  index mapped_player_id (mapped_player_id)
);
-- #endregion

-- #region build counts
create table build_count_snapshot_point (
  id bigint unsigned primary key not null auto_increment,
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp)
);

create table build_count_snapshot (
  snapshot_point_id bigint unsigned not null references build_count_snapshot_point(id),
  mapped_player_id bigint unsigned not null references uuid_mapping(mapped_id),
  value bigint unsigned not null,
  primary key (snapshot_point_id, mapped_player_id),
  index mapped_player_id (mapped_player_id)
);

create table build_count_diff_point (
  id bigint unsigned primary key not null auto_increment,
  root_snapshot_point_id bigint unsigned not null references build_count_snapshot_point(id),
  previous_diff_point_id bigint unsigned null references build_count_diff_point(id),
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp),
  index root_id_then_timestamp (root_snapshot_point_id, record_timestamp)
);

create table build_count_diff (
  diff_point_id bigint unsigned not null references build_count_diff_point(id),
  mapped_player_id bigint unsigned not null references uuid_mapping(mapped_id),
  new_value bigint unsigned not null,
  primary key (diff_point_id, mapped_player_id),
  index mapped_player_id (mapped_player_id)
);
-- #endregion

-- #region play ticks
create table play_ticks_snapshot_point (
  id bigint unsigned primary key not null auto_increment,
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp)
);

create table play_ticks_snapshot (
  snapshot_point_id bigint unsigned not null references play_ticks_snapshot_point(id),
  mapped_player_id bigint unsigned not null references uuid_mapping(mapped_id),
  value bigint unsigned not null,
  primary key (snapshot_point_id, mapped_player_id),
  index mapped_player_id (mapped_player_id)
);

create table play_ticks_diff_point (
  id bigint unsigned primary key not null auto_increment,
  root_snapshot_point_id bigint unsigned not null references play_ticks_snapshot_point(id),
  previous_diff_point_id bigint unsigned null references play_ticks_diff_point(id),
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp),
  index root_id_then_timestamp (root_snapshot_point_id, record_timestamp)
);

create table play_ticks_diff (
  diff_point_id bigint unsigned not null references play_ticks_diff_point(id),
  mapped_player_id bigint unsigned not null references uuid_mapping(mapped_id),
  new_value bigint unsigned not null,
  primary key (diff_point_id, mapped_player_id),
  index mapped_player_id (mapped_player_id)
);
-- #endregion

-- #region vote counts
create table vote_count_snapshot_point (
  id bigint unsigned primary key not null auto_increment,
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp)
);

create table vote_count_snapshot (
  snapshot_point_id bigint unsigned not null references vote_count_snapshot_point(id),
  mapped_player_id bigint unsigned not null references uuid_mapping(mapped_id),
  value bigint unsigned not null,
  primary key (snapshot_point_id, mapped_player_id),
  index mapped_player_id (mapped_player_id)
);

create table vote_count_diff_point (
  id bigint unsigned primary key not null auto_increment,
  root_snapshot_point_id bigint unsigned not null references vote_count_snapshot_point(id),
  previous_diff_point_id bigint unsigned null references vote_count_diff_point(id),
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp),
  index root_id_then_timestamp (root_snapshot_point_id, record_timestamp)
);

create table vote_count_diff (
  diff_point_id bigint unsigned not null references vote_count_diff_point(id),
  mapped_player_id bigint unsigned not null references uuid_mapping(mapped_id),
  new_value bigint unsigned not null,
  primary key (diff_point_id, mapped_player_id),
  index mapped_player_id (mapped_player_id)
);
-- #endregion
