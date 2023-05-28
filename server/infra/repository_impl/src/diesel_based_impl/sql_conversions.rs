use crate::structures_embedded_in_rdb::DiffPointId;
use diesel::backend::{Backend, RawValue};
use diesel::deserialize::FromSql;
use diesel::serialize::ToSql;
use diesel::sql_types::BigInt;
use diesel::sql_types::Unsigned;

impl<B: Backend> ToSql<Unsigned<BigInt>, B> for DiffPointId
where
    u64: ToSql<Unsigned<BigInt>, B>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, B>,
    ) -> diesel::serialize::Result {
        ToSql::<Unsigned<BigInt>, B>::to_sql(&self.0, out)
    }
}

impl<B: Backend> FromSql<Unsigned<BigInt>, B> for DiffPointId
where
    u64: FromSql<Unsigned<BigInt>, B>,
{
    fn from_sql(bytes: RawValue<'_, B>) -> diesel::deserialize::Result<Self> {
        <u64 as FromSql<Unsigned<BigInt>, B>>::from_sql(bytes).map(DiffPointId)
    }
}
