fn main() {
    let conn = duckdb::Connection::open_with_flags(
        "/Users/kimiaki.hashimoto/.tdwh/db.backups/app.db",
        duckdb::Config::default(),
    )
    .unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, target_id, cron_expr, enabled FROM _tdw.scheduled_jobs ORDER BY created_at"
    ).unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })
        .unwrap();
    for row in rows {
        let (id, name, target_id, cron_expr, enabled) = row.unwrap();
        println!("id={id} name={name} target_id={target_id} cron={cron_expr} enabled={enabled}");
    }
}
