#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::*;
use chrono::Utc;
use message::Message;
use message_log::{append_message, read_messages};
use tempfile::TempDir;

#[test]
fn the_data_dir_is_the_base_itself_there_is_no_session_no_uuid_child_no_current_symlink() {
    let base = TempDir::new().expect("temp base");
    let dir = durable_data_dir(base.path()).expect("data dir");
    assert_eq!(
        dir,
        base.path(),
        "the app is not a web app — data lives flat in the base, not behind a minted session"
    );
    assert!(!base.path().join("current").exists());
    let msg = Message {
        from: "owner:human".to_string(),
        text: "persisted message".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    append_message(&dir, &msg).expect("append");
    let again = durable_data_dir(base.path()).expect("data dir again");
    assert_eq!(dir, again);
    let messages = read_messages(&again).expect("read");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "persisted message");
}

#[test]
fn a_legacy_session_dir_behind_the_current_symlink_migrates_its_files_up_then_disappears() {
    let base = TempDir::new().expect("temp base");
    let legacy = base.path().join("0d5e11aa-legacy-session");
    std::fs::create_dir_all(&legacy).expect("legacy session dir");
    let msg = Message {
        from: "owner:human".to_string(),
        text: "message from the session era".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    append_message(&legacy, &msg).expect("append into legacy");
    std::os::unix::fs::symlink(&legacy, base.path().join("current")).expect("current symlink");

    let dir = durable_data_dir(base.path()).expect("data dir");

    assert_eq!(dir, base.path());
    let messages = read_messages(&dir).expect("read migrated");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "message from the session era");
    assert!(
        !base.path().join("current").exists(),
        "the current symlink is a session artifact — gone after migration"
    );
    assert!(
        !legacy.exists(),
        "the emptied legacy session dir does not linger"
    );
}

fn work_dir(parent: &TempDir, name: &str) -> std::path::PathBuf {
    let dir = parent.path().join(name);
    std::fs::create_dir_all(&dir).expect("mkdir work dir");
    dir
}

#[test]
fn a_dir_registered_once_holds_its_id_in_its_own_marker_and_reclaims_it_forever() {
    let base = TempDir::new().expect("registry base");
    let parent = TempDir::new().expect("workspace parent");
    let dir = work_dir(&parent, "my-project");

    let first = register_dir_user_in_base(base.path(), &dir).expect("mint");
    assert!(!first.id.is_empty());
    assert_eq!(
        first.name, "my-project",
        "the name defaults to the folder basename at mint — a self-describing label until renamed in the owner-conversation"
    );

    let second = register_dir_user_in_base(base.path(), &dir).expect("reclaim");
    assert_eq!(
        first, second,
        "the {{id, name}} lives IN the agent's own directory — a second registration reclaims the same identity, never mints a new one"
    );

    assert!(
        dir.join(DIR_USER_ID_MARKER).exists(),
        "the uuid lives in the directory the agent starts in — the dir IS the agent, so many agents on one machine each carry their own identity with no central registry to negotiate over"
    );
    assert!(
        !base.path().join("users").join("by-dir").exists(),
        "the data-home registry gains NO entry — a home-scoped registry would make the machine the one agent"
    );
}

#[test]
fn two_distinct_dirs_mint_distinct_ids() {
    let base = TempDir::new().expect("registry base");
    let parent = TempDir::new().expect("workspace parent");
    let dir_a = work_dir(&parent, "project-a");
    let dir_b = work_dir(&parent, "project-b");

    let a = register_dir_user_in_base(base.path(), &dir_a).expect("mint a");
    let b = register_dir_user_in_base(base.path(), &dir_b).expect("mint b");
    assert_ne!(
        a.id, b.id,
        "two agents started in different dirs are distinct instances — the dir is the identity key"
    );
}

#[cfg(unix)]
#[test]
fn a_symlinked_path_to_the_same_dir_resolves_the_same_id_via_canonicalization() {
    let base = TempDir::new().expect("registry base");
    let parent = TempDir::new().expect("workspace parent");
    let dir = work_dir(&parent, "real-project");
    let minted = register_dir_user_in_base(base.path(), &dir).expect("mint via real path");

    let link = parent.path().join("alias");
    std::os::unix::fs::symlink(&dir, &link).expect("symlink alias -> real-project");

    let via_link = registered_dir_user_in_base(base.path(), &link)
        .expect("a symlinked variant of the same dir resolves the already-registered id");
    assert_eq!(
        via_link.id, minted.id,
        "a symlinked/aliased spelling of the same dir reads the same marker — one dir, one identity"
    );
}

#[test]
fn a_concurrent_register_race_lets_every_loser_reclaim_the_winners_id() {
    let base = TempDir::new().expect("registry base");
    let parent = TempDir::new().expect("workspace parent");

    for round in 0..50 {
        let dir = work_dir(&parent, &format!("contended-{round}"));
        let ids: Vec<String> = std::thread::scope(|scope| {
            let base_path = base.path();
            let racers: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        register_dir_user_in_base(base_path, &dir)
                            .expect("every racer resolves an id")
                            .id
                    })
                })
                .collect();
            racers
                .into_iter()
                .map(|racer| racer.join().expect("racer thread"))
                .collect()
        });
        assert!(
            ids.windows(2).all(|pair| pair[0] == pair[1]),
            "the mint publishes atomically (staged write + hard_link) — every concurrent register in the same dir resolves the ONE winner's id, never a fork and never a partial read; got {ids:?}"
        );
    }
}

#[test]
fn a_read_of_an_unregistered_clean_dir_mints_nothing_and_leaves_no_residue() {
    let base = TempDir::new().expect("registry base");
    let parent = TempDir::new().expect("workspace parent");
    let dir = work_dir(&parent, "just-passing-through");

    let resolved = registered_dir_user_in_base(base.path(), &dir);
    assert!(
        resolved.is_none(),
        "a READ (send / bridge startup) from an unregistered clean dir resolves NO identity — it never mints one as a side effect"
    );
    assert!(
        !dir.join(DIR_USER_ID_MARKER).exists() && !dir.join(".bpmux-session-id").exists(),
        "the read leaves NO marker file in the cwd — this is the anti-pollution guarantee (arbitrary CLI verbs stop scattering identities across repo dirs)"
    );
    assert!(
        !base.path().join("users").join("by-dir").exists(),
        "the read writes NO registry entry either — nothing is created anywhere on a clean-dir read"
    );
}

#[test]
fn a_marker_in_the_dir_is_the_identity_and_stays_exactly_where_it_is() {
    let base = TempDir::new().expect("registry base");
    let parent = TempDir::new().expect("workspace parent");
    let dir = work_dir(&parent, "marked");
    let marker = dir.join(DIR_USER_ID_MARKER);
    std::fs::write(
        &marker,
        serde_json::to_string(&DirUser {
            id: "kept-uuid-1234".to_string(),
            name: "kept-name".to_string(),
        })
        .unwrap(),
    )
    .expect("seed marker");

    let resolved = registered_dir_user_in_base(base.path(), &dir)
        .expect("the dir's own marker resolves its identity");
    assert_eq!(
        resolved.id, "kept-uuid-1234",
        "the marker IS the identity — resolved verbatim, never re-minted"
    );
    assert_eq!(resolved.name, "kept-name", "the stored name carries over");
    assert!(
        marker.exists(),
        "the marker stays in the dir — it is the durable home of the identity, not a legacy artifact to absorb"
    );
    assert!(
        !base.path().join("users").join("by-dir").exists(),
        "no central registry entry is created from a marker read"
    );
}

#[test]
fn a_legacy_central_registry_entry_migrates_into_the_dir_marker_then_disappears() {
    let base = TempDir::new().expect("registry base");
    let parent = TempDir::new().expect("workspace parent");
    let dir = work_dir(&parent, "centrally-registered");
    std::fs::create_dir_all(base.path().join("users").join("by-dir")).expect("registry dir");
    let canonical = std::fs::canonicalize(&dir).expect("canonical");
    let key: String = ring::digest::digest(
        &ring::digest::SHA256,
        canonical.to_string_lossy().as_bytes(),
    )
    .as_ref()
    .iter()
    .map(|byte| format!("{byte:02x}"))
    .collect();
    let entry_path = base
        .path()
        .join("users")
        .join("by-dir")
        .join(format!("{key}.json"));
    std::fs::write(
        &entry_path,
        format!(
            "{{\"path\":\"{}\",\"id\":\"central-uuid-9\",\"name\":\"held-name\"}}",
            canonical.to_string_lossy()
        ),
    )
    .expect("seed central entry");

    let resolved = registered_dir_user_in_base(base.path(), &dir)
        .expect("a legacy central entry still resolves the dir's id");
    assert_eq!(
        resolved.id, "central-uuid-9",
        "the id minted under the central-registry era is preserved verbatim — identity never changes across the migration"
    );
    assert!(
        dir.join(DIR_USER_ID_MARKER).exists(),
        "the identity moves home: the central entry is rewritten as the dir's own marker"
    );
    assert!(
        !entry_path.exists(),
        "the central entry is deleted once the marker is written — the migration finishes, leaving nothing behind"
    );
    let reread = registered_dir_user_in_base(base.path(), &dir).expect("marker resolves");
    assert_eq!(reread.id, "central-uuid-9");
}

#[test]
fn a_legacy_session_id_marker_with_a_raw_uuid_is_imported_then_deleted() {
    let base = TempDir::new().expect("registry base");
    let parent = TempDir::new().expect("workspace parent");
    let dir = work_dir(&parent, "pinned-agent-dir");
    let old_marker = dir.join(".bpmux-session-id");
    std::fs::write(&old_marker, "pinned-uuid-5678").expect("seed old raw-uuid marker");

    let imported = registered_dir_user_in_base(base.path(), &dir)
        .expect("the legacy .bpmux-session-id filename still resolves its id");
    assert_eq!(
        imported.id, "pinned-uuid-5678",
        "Ryan's existing agent dirs pin identity in the OLD marker filename with a raw uuid — the id is preserved, never re-minted"
    );
    assert_eq!(
        imported.name, "pinned-agent-dir",
        "a bare-uuid marker derives its name from the folder basename"
    );
    assert!(
        dir.join(DIR_USER_ID_MARKER).exists(),
        "the old filename is renamed into the current marker — the identity stays home in the dir"
    );
    assert!(
        !old_marker.exists(),
        "the legacy filename is deleted after the rename — no residue"
    );
}

#[test]
fn both_markers_present_and_divergent_prefers_the_new_and_deletes_the_old() {
    let base = TempDir::new().expect("registry base");
    let parent = TempDir::new().expect("workspace parent");
    let dir = work_dir(&parent, "divergent");
    std::fs::write(
        dir.join(DIR_USER_ID_MARKER),
        serde_json::to_string(&DirUser {
            id: "new-authoritative".to_string(),
            name: "new-name".to_string(),
        })
        .unwrap(),
    )
    .expect("seed new marker");
    std::fs::write(dir.join(".bpmux-session-id"), "old-divergent-id").expect("seed old marker");

    let imported = registered_dir_user_in_base(base.path(), &dir).expect("import");
    assert_eq!(
        imported.id, "new-authoritative",
        "when both markers exist and disagree, the new-format .bpmux-user-id is authoritative"
    );
    assert!(
        dir.join(DIR_USER_ID_MARKER).exists(),
        "the authoritative marker stays — it is the identity's home"
    );
    assert!(
        !dir.join(".bpmux-session-id").exists(),
        "the divergent old filename is discarded, not left as residue"
    );
}

#[test]
fn register_imports_an_existing_marker_rather_than_minting_a_fresh_id() {
    let base = TempDir::new().expect("registry base");
    let parent = TempDir::new().expect("workspace parent");
    let dir = work_dir(&parent, "register-over-marker");
    std::fs::write(
        dir.join(DIR_USER_ID_MARKER),
        serde_json::to_string(&DirUser {
            id: "preexisting-id".to_string(),
            name: "preexisting".to_string(),
        })
        .unwrap(),
    )
    .expect("seed marker");

    let registered = register_dir_user_in_base(base.path(), &dir).expect("register");
    assert_eq!(
        registered.id, "preexisting-id",
        "the register (mint) variant reclaims a marker id verbatim — never overwritten with a fresh mint"
    );
    assert!(
        dir.join(DIR_USER_ID_MARKER).exists(),
        "the marker stays in place — register reads it, never absorbs it"
    );
}
