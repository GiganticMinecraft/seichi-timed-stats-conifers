CREATE TABLE conifers.uuid_mapping (
    mapped_id bigint unsigned primary key not null auto_increment,
    uuid char(36) NOT NULL,
    index uuid (uuid)
);

create table conifers.break_count_snapshot_point (
    id bigint unsigned primary key not null auto_increment,
    record_timestamp datetime not null,
    index record_timestamp (record_timestamp)
);

create table conifers.break_count_diff_tree_node (
    id bigint unsigned primary key not null auto_increment,
    root_snapshot_id bigint unsigned not null,
    previous_node_id bigint unsigned null references conifers.break_count_diff_tree_node(id),
    record_timestamp datetime not null,
    index record_timestamp (record_timestamp),
    constraint previous_node_exists
        foreign key (previous_node_id) references conifers.break_count_diff_tree_node(id)
            on delete cascade
            on update restrict,
    constraint root_snapshot_id_references_snapshot_point_id
        foreign key (root_snapshot_id) references conifers.break_count_snapshot_point(id)
            on delete cascade
            on update restrict
);

create table conifers.break_count_diff (
    node_id bigint unsigned not null,
    mapped_player_id bigint unsigned not null,
    primary key (node_id, mapped_player_id),

    new_value bigint unsigned not null,

    constraint node_id_exists
        foreign key (node_id) references conifers.break_count_diff_tree_node(id)
            on delete cascade
            on update restrict,
    constraint player_id_exists
        foreign key (mapped_player_id) references conifers.uuid_mapping(mapped_id)
            on delete cascade
            on update restrict
);
