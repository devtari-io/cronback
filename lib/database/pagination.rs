use proto::common::{PaginationIn, PaginationOut};
use sea_orm::{
    ColumnTrait,
    EntityTrait,
    ModelTrait,
    QueryFilter,
    QueryOrder,
    QuerySelect,
    Select,
};

// Should be implemented for models that provide pagination cursor.
pub trait PaginatedEntity: EntityTrait {
    fn cursor_column() -> Self::Column;
}

pub trait PaginatedSelect<E: EntityTrait> {
    fn with_pagination(self, pagination: &PaginationIn) -> Select<E>;
}

impl<E> PaginatedSelect<E> for Select<E>
where
    E: EntityTrait + PaginatedEntity,
{
    fn with_pagination(self, pagination: &PaginationIn) -> Select<E> {
        let cursor_column = E::cursor_column();
        let mut query = self
            .order_by_desc(cursor_column)
            // Trick. We want to know if there is a next page, so we ask for one
            // more
            .limit(Some(pagination.paginated_query_limit()));

        if let Some(ref cursor) = pagination.cursor {
            query = query.filter(cursor_column.lte(cursor));
        }
        query
    }
}

#[derive(Debug)]
pub struct PaginatedResponse<T> {
    pub pagination: PaginationOut,
    pub data: Vec<T>,
}

impl<T> PaginatedResponse<T> {
    pub fn from(data: Vec<T>, pagination: PaginationOut) -> Self {
        Self { data, pagination }
    }
}

impl<T> PaginatedResponse<T>
where
    T: ModelTrait,
    T::Entity: PaginatedEntity,
{
    pub fn paginate(mut results: Vec<T>, pagination: &PaginationIn) -> Self {
        // 1. Clip the bottom of results
        //
        // Despite only adding 1 to the limit at the time of query, we
        // can't trust if this will remain true in the future. So we
        // clip the result to the limit.
        let next_cursor = {
            // drain panics if we slice outside the result boundaries
            let clip = std::cmp::min(results.len(), pagination.limit());
            let mut drained = results.drain(clip..);
            drained.next()
        };
        // 2. Set the has_more flag to true
        let has_more = next_cursor.is_some();
        let cursor_column = <T::Entity as PaginatedEntity>::cursor_column();

        let pagination_out = PaginationOut {
            next_cursor: next_cursor.map(|x| {
                x.get(cursor_column)
                    .expect("Cursor column is not string convertible!")
            }),
            has_more,
        };

        Self {
            pagination: pagination_out,
            data: results,
        }
    }
}
