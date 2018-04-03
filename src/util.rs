use tokio_postgres::types::ToSql;

/// Filtering operation
#[derive(Clone, Copy, Debug)]
pub enum FilteredOperation {
    Select,
    Delete,
}

/// Construct a simple select or delete query.
pub struct FilteredOperationBuilder {
    op: FilteredOperation,
    table: String,
    extra: String,
    args: Vec<(String, Box<ToSql + Send + 'static>)>,
}

impl FilteredOperationBuilder {
    /// Create a new builder
    pub fn new<N: Into<String>>(op: FilteredOperation, table: N) -> Self {
        Self {
            op,
            table: table.into(),
            extra: Default::default(),
            args: Default::default(),
        }
    }

    /// Add filtering arguments
    pub fn with_arg<C: Into<String>, V: ToSql + Send + 'static>(mut self, column: C, value: V) -> Self {
        self.args.push((column.into(), Box::new(value)));
        self
    }

    /// Add additional statements before the semicolon
    pub fn with_extra<S: Into<String>>(mut self, statements: S) -> Self {
        self.extra = statements.into();
        self
    }

    /// Build a query
    pub fn build(self) -> (String, Vec<Box<ToSql + Send + 'static>>) {
        let mut args = vec![];
        let mut query = format!(
            "{} {}",
            match self.op {
                FilteredOperation::Select => "SELECT * FROM",
                FilteredOperation::Delete => "DELETE FROM",
            },
            self.table.to_string()
        );

        for (i, (col, arg)) in self.args.into_iter().enumerate() {
            if i == 0 {
                query.push_str(" WHERE ");
            } else {
                query.push_str(" AND ");
            }
            query.push_str(&format!("{} = ${}", col, i + 1));
            args.push(arg);
        }
        let out = format!("{} {};", &query, self.extra);

        (out, args)
    }
}

/// Construct a simple insert query.
pub struct InsertBuilder {
    table: String,
    extra: String,
    values: Vec<(String, Box<ToSql + Send + 'static>)>,
}

impl InsertBuilder {
    pub fn new<N: Into<String>>(table: N) -> Self {
        Self {
            table: table.into(),
            extra: Default::default(),
            values: Default::default(),
        }
    }

    pub fn with_arg<K: Into<String>, V: ToSql + Send + 'static>(mut self, k: K, v: V) -> Self {
        self.values.push((k.into(), Box::new(v)));
        self
    }

    pub fn with_extra<S: Into<String>>(mut self, s: S) -> Self {
        self.extra = s.into();
        self
    }

    /// Builds a query
    pub fn build(self) -> (String, Vec<Box<ToSql + Send + 'static>>) {
        let mut args = vec![];
        let mut query = format!("INSERT INTO {}", self.table.to_string());

        let mut col_string = String::new();
        let mut arg_string = String::new();
        for (i, (col, arg)) in self.values.into_iter().enumerate() {
            if i > 0 {
                col_string.push_str(", ");
                arg_string.push_str(", ");
            }

            col_string.push_str(&col);
            arg_string.push_str(&format!("${}", i + 1));
            args.push(arg);
        }
        query = format!("{} ({}) VALUES ({})", &query, &col_string, &arg_string);

        let out = format!("{} {};", &query, self.extra);

        (out, args)
    }
}
