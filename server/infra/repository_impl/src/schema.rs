// @generated automatically by Diesel CLI.

diesel::table! {
    break_count_diff (diff_point_id, player_uuid) {
        diff_point_id -> Unsigned<Bigint>,
        player_uuid -> Char,
        new_value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    break_count_diff_point (id) {
        id -> Unsigned<Bigint>,
        root_full_snapshot_point_id -> Unsigned<Bigint>,
        previous_diff_point_id -> Nullable<Unsigned<Bigint>>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    break_count_full_snapshot (full_snapshot_point_id, player_uuid) {
        full_snapshot_point_id -> Unsigned<Bigint>,
        player_uuid -> Char,
        value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    break_count_full_snapshot_point (id) {
        id -> Unsigned<Bigint>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    build_count_diff (diff_point_id, player_uuid) {
        diff_point_id -> Unsigned<Bigint>,
        player_uuid -> Char,
        new_value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    build_count_diff_point (id) {
        id -> Unsigned<Bigint>,
        root_full_snapshot_point_id -> Unsigned<Bigint>,
        previous_diff_point_id -> Nullable<Unsigned<Bigint>>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    build_count_full_snapshot (full_snapshot_point_id, player_uuid) {
        full_snapshot_point_id -> Unsigned<Bigint>,
        player_uuid -> Char,
        value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    build_count_full_snapshot_point (id) {
        id -> Unsigned<Bigint>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    play_ticks_diff (diff_point_id, player_uuid) {
        diff_point_id -> Unsigned<Bigint>,
        player_uuid -> Char,
        new_value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    play_ticks_diff_point (id) {
        id -> Unsigned<Bigint>,
        root_full_snapshot_point_id -> Unsigned<Bigint>,
        previous_diff_point_id -> Nullable<Unsigned<Bigint>>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    play_ticks_full_snapshot (full_snapshot_point_id, player_uuid) {
        full_snapshot_point_id -> Unsigned<Bigint>,
        player_uuid -> Char,
        value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    play_ticks_full_snapshot_point (id) {
        id -> Unsigned<Bigint>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    vote_count_diff (diff_point_id, player_uuid) {
        diff_point_id -> Unsigned<Bigint>,
        player_uuid -> Char,
        new_value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    vote_count_diff_point (id) {
        id -> Unsigned<Bigint>,
        root_full_snapshot_point_id -> Unsigned<Bigint>,
        previous_diff_point_id -> Nullable<Unsigned<Bigint>>,
        record_timestamp -> Datetime,
    }
}

diesel::table! {
    vote_count_full_snapshot (full_snapshot_point_id, player_uuid) {
        full_snapshot_point_id -> Unsigned<Bigint>,
        player_uuid -> Char,
        value -> Unsigned<Bigint>,
    }
}

diesel::table! {
    vote_count_full_snapshot_point (id) {
        id -> Unsigned<Bigint>,
        record_timestamp -> Datetime,
    }
}

diesel::joinable!(break_count_diff -> break_count_diff_point (diff_point_id));
diesel::joinable!(break_count_diff_point -> break_count_full_snapshot_point (root_full_snapshot_point_id));
diesel::joinable!(break_count_full_snapshot -> break_count_full_snapshot_point (full_snapshot_point_id));
diesel::joinable!(build_count_diff -> build_count_diff_point (diff_point_id));
diesel::joinable!(build_count_diff_point -> build_count_full_snapshot_point (root_full_snapshot_point_id));
diesel::joinable!(build_count_full_snapshot -> build_count_full_snapshot_point (full_snapshot_point_id));
diesel::joinable!(play_ticks_diff -> play_ticks_diff_point (diff_point_id));
diesel::joinable!(play_ticks_diff_point -> play_ticks_full_snapshot_point (root_full_snapshot_point_id));
diesel::joinable!(play_ticks_full_snapshot -> play_ticks_full_snapshot_point (full_snapshot_point_id));
diesel::joinable!(vote_count_diff -> vote_count_diff_point (diff_point_id));
diesel::joinable!(vote_count_diff_point -> vote_count_full_snapshot_point (root_full_snapshot_point_id));
diesel::joinable!(vote_count_full_snapshot -> vote_count_full_snapshot_point (full_snapshot_point_id));

diesel::allow_tables_to_appear_in_same_query!(
    break_count_diff,
    break_count_diff_point,
    break_count_full_snapshot,
    break_count_full_snapshot_point,
    build_count_diff,
    build_count_diff_point,
    build_count_full_snapshot,
    build_count_full_snapshot_point,
    play_ticks_diff,
    play_ticks_diff_point,
    play_ticks_full_snapshot,
    play_ticks_full_snapshot_point,
    vote_count_diff,
    vote_count_diff_point,
    vote_count_full_snapshot,
    vote_count_full_snapshot_point,
);
