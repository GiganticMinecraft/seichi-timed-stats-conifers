// @generated automatically by Diesel CLI.

diesel::table! {
    break_count_diff (diff_point_id, mapped_player_id) {
        diff_point_id -> Unsigned<Bigint>,
        mapped_player_id -> Unsigned<Bigint>,
        new_value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    break_count_diff_point (id) {
        id -> Unsigned<Bigint>,
        root_snapshot_point_id -> Unsigned<Bigint>,
        previous_diff_point_id -> Nullable<Unsigned<Bigint>>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    break_count_snapshot (snapshot_point_id, mapped_player_id) {
        snapshot_point_id -> Unsigned<Bigint>,
        mapped_player_id -> Unsigned<Bigint>,
        value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    break_count_snapshot_point (id) {
        id -> Unsigned<Bigint>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    build_count_diff (diff_point_id, mapped_player_id) {
        diff_point_id -> Unsigned<Bigint>,
        mapped_player_id -> Unsigned<Bigint>,
        new_value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    build_count_diff_point (id) {
        id -> Unsigned<Bigint>,
        root_snapshot_point_id -> Unsigned<Bigint>,
        previous_diff_point_id -> Nullable<Unsigned<Bigint>>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    build_count_snapshot (snapshot_point_id, mapped_player_id) {
        snapshot_point_id -> Unsigned<Bigint>,
        mapped_player_id -> Unsigned<Bigint>,
        value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    build_count_snapshot_point (id) {
        id -> Unsigned<Bigint>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    play_ticks_diff (diff_point_id, mapped_player_id) {
        diff_point_id -> Unsigned<Bigint>,
        mapped_player_id -> Unsigned<Bigint>,
        new_value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    play_ticks_diff_point (id) {
        id -> Unsigned<Bigint>,
        root_snapshot_point_id -> Unsigned<Bigint>,
        previous_diff_point_id -> Nullable<Unsigned<Bigint>>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    play_ticks_snapshot (snapshot_point_id, mapped_player_id) {
        snapshot_point_id -> Unsigned<Bigint>,
        mapped_player_id -> Unsigned<Bigint>,
        value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    play_ticks_snapshot_point (id) {
        id -> Unsigned<Bigint>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    uuid_mapping (mapped_id) {
        mapped_id -> Unsigned<Bigint>,
        uuid -> Char,
    }
}

diesel::table! {
    vote_count_diff (diff_point_id, mapped_player_id) {
        diff_point_id -> Unsigned<Bigint>,
        mapped_player_id -> Unsigned<Bigint>,
        new_value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    vote_count_diff_point (id) {
        id -> Unsigned<Bigint>,
        root_snapshot_point_id -> Unsigned<Bigint>,
        previous_diff_point_id -> Nullable<Unsigned<Bigint>>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    vote_count_snapshot (snapshot_point_id, mapped_player_id) {
        snapshot_point_id -> Unsigned<Bigint>,
        mapped_player_id -> Unsigned<Bigint>,
        value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    vote_count_snapshot_point (id) {
        id -> Unsigned<Bigint>,
        record_timestamp -> Datetime,
    }
}

diesel::joinable!(break_count_diff -> break_count_diff_point (diff_point_id));
diesel::joinable!(break_count_diff -> uuid_mapping (mapped_player_id));
diesel::joinable!(break_count_diff_point -> break_count_snapshot_point (root_snapshot_point_id));
diesel::joinable!(break_count_snapshot -> break_count_snapshot_point (snapshot_point_id));
diesel::joinable!(break_count_snapshot -> uuid_mapping (mapped_player_id));
diesel::joinable!(build_count_diff -> build_count_diff_point (diff_point_id));
diesel::joinable!(build_count_diff -> uuid_mapping (mapped_player_id));
diesel::joinable!(build_count_diff_point -> build_count_snapshot_point (root_snapshot_point_id));
diesel::joinable!(build_count_snapshot -> build_count_snapshot_point (snapshot_point_id));
diesel::joinable!(build_count_snapshot -> uuid_mapping (mapped_player_id));
diesel::joinable!(play_ticks_diff -> play_ticks_diff_point (diff_point_id));
diesel::joinable!(play_ticks_diff -> uuid_mapping (mapped_player_id));
diesel::joinable!(play_ticks_diff_point -> play_ticks_snapshot_point (root_snapshot_point_id));
diesel::joinable!(play_ticks_snapshot -> play_ticks_snapshot_point (snapshot_point_id));
diesel::joinable!(play_ticks_snapshot -> uuid_mapping (mapped_player_id));
diesel::joinable!(vote_count_diff -> uuid_mapping (mapped_player_id));
diesel::joinable!(vote_count_diff -> vote_count_diff_point (diff_point_id));
diesel::joinable!(vote_count_diff_point -> vote_count_snapshot_point (root_snapshot_point_id));
diesel::joinable!(vote_count_snapshot -> uuid_mapping (mapped_player_id));
diesel::joinable!(vote_count_snapshot -> vote_count_snapshot_point (snapshot_point_id));

diesel::allow_tables_to_appear_in_same_query!(
    break_count_diff,
    break_count_diff_point,
    break_count_snapshot,
    break_count_snapshot_point,
    build_count_diff,
    build_count_diff_point,
    build_count_snapshot,
    build_count_snapshot_point,
    play_ticks_diff,
    play_ticks_diff_point,
    play_ticks_snapshot,
    play_ticks_snapshot_point,
    uuid_mapping,
    vote_count_diff,
    vote_count_diff_point,
    vote_count_snapshot,
    vote_count_snapshot_point,
);
