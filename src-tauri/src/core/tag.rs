use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

#[derive(Clone)]
pub struct Tag {
    pub id: i32,
    pub name: String,
}

impl Tag {
    pub fn read_all(db: PooledConnection<SqliteConnectionManager>) -> Vec<Tag> {
        let mut stmt = db.prepare("SELECT id, name FROM tags").unwrap();
        stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .unwrap()
        .collect::<Result<Vec<Tag>, _>>()
        .unwrap()
    }
}
