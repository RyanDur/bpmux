use crate::room;

pub(super) fn run_room_spawn(role: &str, agent: &str, headless: bool) -> anyhow::Result<()> {
    let storage = room::FileRoomStorage::from_env();
    let pid = if headless {
        room::spawn_captured(agent, &[], &storage.output_path(role))?
    } else {
        room::spawn_pty(agent, &[])?
    };
    let entry = room::Room {
        role: role.to_string(),
        agent: agent.to_string(),
        pid,
        created_at: chrono::Utc::now(),
    };
    <room::FileRoomStorage as room::RoomStore>::save(&storage, &entry)?;
    Ok(())
}

pub(super) fn run_room_list() -> anyhow::Result<()> {
    let storage = room::FileRoomStorage::from_env();
    let mut rooms = <room::FileRoomStorage as room::RoomStore>::list(&storage)?;
    rooms.sort_by(|a, b| a.role.cmp(&b.role));
    for room in &rooms {
        println!("{}\t{}\t{}", room.role, room.agent, room.pid);
    }
    Ok(())
}

pub(super) fn run_room_read(role: &str, lines: Option<usize>) -> anyhow::Result<()> {
    let storage = room::FileRoomStorage::from_env();
    let raw = <room::FileRoomStorage as room::RoomStore>::read(&storage, role, lines)?;
    print!("{}", room::strip_ansi(&raw));
    Ok(())
}

pub(super) fn run_room_kill(role: &str) -> anyhow::Result<()> {
    let storage = room::FileRoomStorage::from_env();
    let entry = <room::FileRoomStorage as room::RoomStore>::get(&storage, role)?;
    room::terminate(entry.pid)?;
    Ok(())
}
