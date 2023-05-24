-- #region break counts
create table break_count_full_snapshot_point (
  id bigint unsigned primary key not null auto_increment,
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp)
);

create table break_count_full_snapshot (
  full_snapshot_point_id bigint unsigned not null references break_count_full_snapshot_point(id),
  player_uuid char(36) not null,
  value bigint unsigned not null,
  primary key (full_snapshot_point_id, player_uuid),
  index player_uuid (player_uuid)
);

create table break_count_diff_point (
  id bigint unsigned primary key not null auto_increment,
  root_full_snapshot_point_id bigint unsigned not null references break_count_full_snapshot_point(id),
  previous_diff_point_id bigint unsigned null references break_count_diff_point(id),
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp),
  index root_id_then_timestamp (root_full_snapshot_point_id, record_timestamp)
);

create table break_count_diff (
  diff_point_id bigint unsigned not null references break_count_diff_point(id),
  player_uuid char(36) not null,
  new_value bigint unsigned not null,
  primary key (diff_point_id, player_uuid),
  index player_uuid (player_uuid)
);
-- #endregion

-- #region build counts
create table build_count_full_snapshot_point (
  id bigint unsigned primary key not null auto_increment,
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp)
);

create table build_count_full_snapshot (
  full_snapshot_point_id bigint unsigned not null references build_count_full_snapshot_point(id),
  player_uuid char(36) not null,
  value bigint unsigned not null,
  primary key (full_snapshot_point_id, player_uuid),
  index player_uuid (player_uuid)
);

create table build_count_diff_point (
  id bigint unsigned primary key not null auto_increment,
  root_full_snapshot_point_id bigint unsigned not null references build_count_full_snapshot_point(id),
  previous_diff_point_id bigint unsigned null references build_count_diff_point(id),
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp),
  index root_id_then_timestamp (root_full_snapshot_point_id, record_timestamp)
);

create table build_count_diff (
  diff_point_id bigint unsigned not null references build_count_diff_point(id),
  player_uuid char(36) not null,
  new_value bigint unsigned not null,
  primary key (diff_point_id, player_uuid),
  index player_uuid (player_uuid)
);
-- #endregion

-- #region play ticks
create table play_ticks_full_snapshot_point (
  id bigint unsigned primary key not null auto_increment,
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp)
);

create table play_ticks_full_snapshot (
  full_snapshot_point_id bigint unsigned not null references play_ticks_full_snapshot_point(id),
  player_uuid char(36) not null,
  value bigint unsigned not null,
  primary key (full_snapshot_point_id, player_uuid),
  index player_uuid (player_uuid)
);

create table play_ticks_diff_point (
  id bigint unsigned primary key not null auto_increment,
  root_full_snapshot_point_id bigint unsigned not null references play_ticks_full_snapshot_point(id),
  previous_diff_point_id bigint unsigned null references play_ticks_diff_point(id),
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp),
  index root_id_then_timestamp (root_full_snapshot_point_id, record_timestamp)
);

create table play_ticks_diff (
  diff_point_id bigint unsigned not null references play_ticks_diff_point(id),
  player_uuid char(36) not null,
  new_value bigint unsigned not null,
  primary key (diff_point_id, player_uuid),
  index player_uuid (player_uuid)
);
-- #endregion

-- #region vote counts
create table vote_count_full_snapshot_point (
  id bigint unsigned primary key not null auto_increment,
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp)
);

create table vote_count_full_snapshot (
  full_snapshot_point_id bigint unsigned not null references vote_count_full_snapshot_point(id),
  player_uuid char(36) not null,
  value bigint unsigned not null,
  primary key (full_snapshot_point_id, player_uuid),
  index player_uuid (player_uuid)
);

create table vote_count_diff_point (
  id bigint unsigned primary key not null auto_increment,
  root_full_snapshot_point_id bigint unsigned not null references vote_count_full_snapshot_point(id),
  previous_diff_point_id bigint unsigned null references vote_count_diff_point(id),
  record_timestamp datetime not null,
  index record_timestamp (record_timestamp),
  index root_id_then_timestamp (root_full_snapshot_point_id, record_timestamp)
);

create table vote_count_diff (
  diff_point_id bigint unsigned not null references vote_count_diff_point(id),
  player_uuid char(36) not null,
  new_value bigint unsigned not null,
  primary key (diff_point_id, player_uuid),
  index player_uuid (player_uuid)
);
-- #endregion
