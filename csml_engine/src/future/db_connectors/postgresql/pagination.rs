use diesel::pg::Pg;
use diesel::query_builder::{AstPass, Query, QueryFragment};
use diesel::QueryResult;
use diesel::sql_types::BigInt;
use diesel_async::{AsyncPgConnection, RunQueryDsl, methods::LoadQuery};

pub trait Paginate: Sized {
    fn paginate(self, page: i64) -> Paginated<Self>;
}

impl<T> Paginate for T {
    fn paginate(self, page: i64) -> Paginated<Self> {
        Paginated {
            query: self,
            per_page: DEFAULT_PER_PAGE,
            offset: (page - 1) * DEFAULT_PER_PAGE,
        }
    }
}

const DEFAULT_PER_PAGE: i64 = 10;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Paginated<T> {
    query: T,
    offset: i64,
    per_page: i64,
}

impl<'a, T: 'a> Paginated<T> {
    pub fn per_page(self, per_page: i64) -> Self {
        let old_page = self.offset / self.per_page + 1;
        Paginated {
            per_page,
            offset: (old_page - 1) * per_page,
            query: self.query,
        }
    }

    pub async fn load_and_count_pages<U>(
        self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<(Vec<U>, i64)>
    where
        Self: LoadQuery<'a, AsyncPgConnection, (U, i64)>,
        U: Send
    {
        let per_page = self.per_page;
        let results = self.load::<(U, i64)>(conn).await?;
        let total = results.get(0).map(|x| x.1).unwrap_or(0);
        let records = results.into_iter().map(|x| x.0).collect();
        let total_pages = (total as f64 / per_page as f64).ceil() as i64;
        Ok((records, total_pages))
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T> QueryFragment<Pg> for Paginated<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.per_page)?;
        out.push_sql(" OFFSET ");
        out.push_bind_param::<BigInt, _>(&self.offset)?;
        Ok(())
    }
}
