use tokio_postgres::types::ToSql;

#[derive(Clone, Copy, Debug)]
pub enum SimpleQueryOperation {
    Select,
    Delete,
}

pub struct SimpleQueryBuilder {
    op: SimpleQueryOperation,
    table: String,
    additions: Vec<(String, Box<ToSql + Send + 'static>)>,
}

impl SimpleQueryBuilder {
    pub fn new<N: Into<String>>(op: SimpleQueryOperation, table: N) -> Self {
        Self {
            op,
            table: table.into(),
            additions: Default::default(),
        }
    }

    pub fn with_arg<K: Into<String>, V: ToSql + Send + 'static>(mut self, k: K, v: V) -> Self {
        self.additions.push((k.into(), Box::new(v)));
        self
    }

    pub fn build(self) -> (String, Vec<Box<ToSql + Send + 'static>>) {
        let mut args = vec![];
        let mut query = format!(
            "{} {}",
            match self.op {
                SimpleQueryOperation::Select => "SELECT * FROM",
                SimpleQueryOperation::Delete => "DELETE FROM",
            },
            self.table.to_string()
        );
        for (i, (col, arg)) in self.additions.into_iter().enumerate() {
            if i == 0 {
                query.push_str(" WHERE ");
            } else {
                query.push_str(" AND ");
            }
            query.push_str(&format!("{} = ${}", col, i));
            args.push(arg);
        }
        query.push(';');

        (query, args)
    }
}
