use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use validator::Validate;

// The query parameters for pagination and cursor management
#[derive(Debug, Deserialize, Default, Validate)]
pub(crate) struct Pagination<T> {
    pub before: Option<T>,
    pub after: Option<T>,
    #[validate(range(
        min = 1,
        max = 100,
        message = "must be between 1 and 100"
    ))]
    pub limit: Option<usize>,
}

impl<T> ::std::convert::From<Pagination<T>>
    for proto::scheduler_proto::Pagination
where
    T: Into<String>,
{
    fn from(value: Pagination<T>) -> Self {
        Self {
            before: value.before.map(Into::into),
            after: value.after.map(Into::into),
            limit: value.limit.map(|x| x as u64),
        }
    }
}

impl<T> Pagination<T> {
    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(20)
    }
}

pub(crate) fn paginate<T, B>(
    mut results: Vec<T>,
    pagination: Pagination<B>,
) -> Value
where
    T: Serialize + Send + 'static,
{
    let mut has_more = false;
    if results.len() > pagination.limit() {
        // 1. Clip the top or the bottom of results based on whether before or
        // after is set.
        //
        // Despite only adding 1 to the limit at the time of query, we
        // can't trust if this will remain true in the future. So we
        // clip the result to the limit.
        while results.len() > pagination.limit() {
            if pagination.before.is_some() {
                results.remove(0);
            } else {
                results.pop();
            }
        }
        // 2. Set the has_more flag to true
        has_more = true;
    }

    json!( {
        "data": results,
        "has_more": has_more,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination() {
        let results = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let pagination: Pagination<i32> = Pagination {
            before: None,
            after: None,
            limit: Some(5),
        };
        let result = paginate(results, pagination);
        // by default we clip the bottom of results (default is after)
        assert_eq!(result["data"].as_array().unwrap().len(), 5);
        assert_eq!(result["data"].as_array().unwrap(), &vec![1, 2, 3, 4, 5]);
        assert_eq!(result["has_more"], true);

        // Test with before
        let results = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let pagination: Pagination<i32> = Pagination {
            before: Some(5), // The actual value doesn't matter.
            after: None,
            limit: Some(5),
        };
        let result = paginate(results, pagination);
        assert_eq!(result["data"].as_array().unwrap().len(), 5);
        assert_eq!(result["data"].as_array().unwrap(), &vec![6, 7, 8, 9, 10]);
        assert_eq!(result["has_more"], true);

        // Test results equal to limit
        let results = vec![1, 2, 3, 4, 5];
        let pagination: Pagination<i32> = Pagination {
            before: Some(5), // The actual value doesn't matter.
            after: None,
            limit: Some(5),
        };
        let result = paginate(results, pagination);
        assert_eq!(result["data"].as_array().unwrap().len(), 5);
        assert_eq!(result["data"].as_array().unwrap(), &vec![1, 2, 3, 4, 5]);
        assert_eq!(result["has_more"], false);

        // Test results less than limit
        let results = vec![1, 2, 3];
        let pagination: Pagination<i32> = Pagination {
            before: Some(5), // The actual value doesn't matter.
            after: None,
            limit: Some(5),
        };
        let result = paginate(results, pagination);
        assert_eq!(result["data"].as_array().unwrap().len(), 3);
        assert_eq!(result["data"].as_array().unwrap(), &vec![1, 2, 3]);
        assert_eq!(result["has_more"], false);
    }

    #[test]
    fn test_pagination_valid() {
        // valid
        let pagination: Pagination<i32> = Pagination {
            before: None,
            after: None,
            limit: Some(5),
        };
        let result = pagination.validate();
        assert!(result.is_ok());
        // valid
        let pagination: Pagination<i32> = Pagination {
            before: Some(1),
            after: None,
            limit: Some(5),
        };
        let result = pagination.validate();
        assert!(result.is_ok());

        // valid
        let pagination: Pagination<i32> = Pagination {
            before: None,
            after: Some(1),
            limit: Some(5),
        };
        let result = pagination.validate();
        assert!(result.is_ok());
        // valid
        let pagination: Pagination<i32> = Pagination {
            before: None,
            after: None,
            limit: None,
        };
        let result = pagination.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_pagination_invalid() {
        // limit cannot be zero
        let pagination: Pagination<i32> = Pagination {
            before: None,
            after: None,
            limit: Some(0),
        };
        let result = pagination.validate();
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "limit: must be between 1 and 100"
        );

        // limit cannot be greater than 100
        let pagination: Pagination<i32> = Pagination {
            before: None,
            after: None,
            limit: Some(101),
        };
        let result = pagination.validate();
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "limit: must be between 1 and 100"
        );
    }
}
