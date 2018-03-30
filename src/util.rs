use tokio_postgres::types::ToSql;

#[derive(Clone, Copy, Debug)]
pub enum SimpleQueryOperation {
    Select,
    Delete,
    Insert,
}

pub struct SimpleQueryBuilder {
    op: SimpleQueryOperation,
    table: String,
    extra: String,
    additions: Vec<(String, Box<ToSql + Send + 'static>)>,
}

impl SimpleQueryBuilder {
    pub fn new<N: Into<String>>(op: SimpleQueryOperation, table: N) -> Self {
        Self {
            op,
            table: table.into(),
            extra: Default::default(),
            additions: Default::default(),
        }
    }

    pub fn with_arg<K: Into<String>, V: ToSql + Send + 'static>(mut self, k: K, v: V) -> Self {
        self.additions.push((k.into(), Box::new(v)));
        self
    }

    pub fn with_extra<S: Into<String>>(mut self, s: S) -> Self {
        self.extra = s.into();
        self
    }

    pub fn build(self) -> (String, Vec<Box<ToSql + Send + 'static>>) {
        let mut args = vec![];
        let mut query = format!(
            "{} {}",
            match self.op {
                SimpleQueryOperation::Select => "SELECT * FROM",
                SimpleQueryOperation::Delete => "DELETE FROM",
                SimpleQueryOperation::Insert => "INSERT INTO",
            },
            self.table.to_string()
        );
        if let SimpleQueryOperation::Insert = self.op {
            let mut col_string = String::new();
            let mut arg_string = String::new();
            for (i, (col, arg)) in self.additions.into_iter().enumerate() {
                if i > 0 {
                    col_string.push_str(", ");
                    arg_string.push_str(", ");
                }

                col_string.push_str(&col);
                arg_string.push_str(&format!("${}", i + 1));
                args.push(arg);
            }
            query = format!("{} ({}) VALUES ({})", &query, &col_string, &arg_string);
        } else {
            for (i, (col, arg)) in self.additions.into_iter().enumerate() {
                if i == 0 {
                    query.push_str(" WHERE ");
                } else {
                    query.push_str(" AND ");
                }
                query.push_str(&format!("{} = ${}", col, i + 1));
                args.push(arg);
            }
        }
        let out = format!("{} {};", &query, self.extra);

        (out, args)
    }
}
