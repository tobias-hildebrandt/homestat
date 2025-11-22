struct SqliteDb;

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, Row};

    #[test]
    fn db_test() {
        let connection = Connection::open("/tmp/test.db3").unwrap();

        #[derive(Debug)]
        struct Test {
            id: u64,
            text: String,
        }

        impl Test {
            fn from_full_row(value: Row<'_>) -> Self {
                Test {
                    id: value.get(0).unwrap(),
                    text: value.get(1).unwrap(),
                }
            }
        }

        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS test123 (
                    id INTEGER PRIMARY KEY AUTOINCREMENT
                    , some_text TEXT NOT NULL
                )",
                (),
            )
            .unwrap();

        for _ in 0..10 {
            let mut s = String::new();
            for _ in 0..rand::random_range(5..=10) {
                s.push(rand::random_range('a'..='z'));
            }

            connection
                .execute("INSERT INTO test123 (some_text) VALUES (?1)", (s,))
                .unwrap();
        }

        let mut statement = connection
            .prepare("SELECT id, some_text FROM test123")
            .unwrap();

        let iter = statement
            .query_map([], |row| {
                Ok(Test {
                    id: row.get(0).unwrap(),
                    text: row.get(1).unwrap(),
                })
            })
            .unwrap();

        for t in iter {
            println!("test: {:?}", t);
        }
    }
}
