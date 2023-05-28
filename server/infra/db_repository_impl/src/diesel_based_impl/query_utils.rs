use diesel::dsl::Limit;
use diesel::QueryResult;
use diesel_async::methods::LoadQuery;
use diesel_async::{return_futures, AsyncConnection, RunQueryDsl};
use futures_util::FutureExt;
use std::future::Future;

// 実質 impl Future<Output = QueryResult<Option<U>>> (trait 内に impl Future は書けないので、型を明示する必要がある)
type RunFirstReturnFuture<'conn, 'query, Q, Conn, U> = futures_util::future::Map<
    return_futures::LoadFuture<'conn, 'query, Q, Conn, U>,
    fn(
        <return_futures::LoadFuture<'conn, 'query, Q, Conn, U> as Future>::Output,
    ) -> QueryResult<Option<U>>,
>;

pub trait RunFirstOptionalDsl<Conn>: Sized + RunQueryDsl<Conn> {
    fn first_optional<'query, 'conn, U>(
        self,
        conn: &'conn mut Conn,
    ) -> RunFirstReturnFuture<'conn, 'query, Limit<Self>, Conn, U>
    where
        U: Send + 'conn,
        Conn: AsyncConnection,
        Self: diesel::query_dsl::methods::LimitDsl,
        Limit<Self>: LoadQuery<'query, Conn, U> + Send + 'query,
    {
        self.limit(1)
            .load::<U>(conn)
            .map(|res| res.map(|vec| vec.into_iter().next()))
    }
}

impl<T, Conn> RunFirstOptionalDsl<Conn> for T where T: RunQueryDsl<Conn> {}
