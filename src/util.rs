use tokio_postgres::types::ToSql;

pub struct SimpleQueryBuilder {
    base: String,
    additions: Vec<(String, Box<ToSql>)>,
}

impl SimpleQueryBuilder {
    pub fn new<B: Into<String>>(base: B) -> Self {
        Self {
            base: base.into(),
            additions: Default::default(),
        }
    }

    pub fn with_arg<K: Into<String>, V: Into<ToSql>>(mut self, k: K, v: V) -> Self {
        self.additions.push((k.into(), v.into()));
        self
    }

    pub fn build(self) -> (String, Vec<ToSql>) {
        let mut args = vec![];
        let mut query = self.base.to_string();
        for (i, (col, arg)) in self.additions.enumerate() {
            if i == 0 {
                query.push_str(" WHERE ");
            } else {
                query.push_str(" AND ");
            }
            query.push_str(&format!("{} = ${}", col, i));
            args.push(&arg);
        }
        query.push(';');

        (query, args)
    }
}
