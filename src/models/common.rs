use chrono::prelude::*;
use stq_db::statement::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetterPayload<T> {
    pub value: T,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ValueContainer<T> {
    pub value: T,
}

impl<T> From<T> for ValueContainer<T> {
    fn from(value: T) -> Self {
        Self { value }
    }
}

pub fn into_range(from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Option<ValueContainer<Range<DateTime<Utc>>>> {
    if let (Some(from), Some(to)) = (from, to) {
        Some(
            Range::Between((
                {
                    RangeLimit {
                        value: from,
                        inclusive: true,
                    }
                },
                {
                    RangeLimit {
                        value: to,
                        inclusive: true,
                    }
                },
            )).into(),
        )
    } else if let Some(value) = from {
        Some(Range::From({ RangeLimit { value, inclusive: true } }).into())
    } else if let Some(value) = to {
        Some(Range::To({ RangeLimit { value, inclusive: true } }).into())
    } else {
        None
    }
}
