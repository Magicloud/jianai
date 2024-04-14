use diesel::{
    deserialize::{ Result as ResultDe, Queryable, FromSql },
    pg::Pg,
    Expression,
    AppearsOnTable,
    query_builder::{ QueryId, QueryFragment },
};
use serde::Serialize;
use crate::schema::sql_types;
use byteorder::{ NetworkEndian, ReadBytesExt };

#[derive(Debug, Clone, Copy, QueryId, Serialize)]
pub struct Box {
    pub point1: Point,
    pub point2: Point,
}
impl FromSql<sql_types::Box, Pg> for Box {
    fn from_sql(
        bytes: <diesel::pg::Pg as diesel::backend::Backend>::RawValue<'_>
    ) -> ResultDe<Self> {
        // `'( x1 , y1 ) , ( x2 , y2 )'` in text
        let mut x1 = &bytes.as_bytes()[0..8];
        let mut y1 = &bytes.as_bytes()[8..16];
        let mut x2 = &bytes.as_bytes()[16..24];
        let mut y2 = &bytes.as_bytes()[24..32];
        let x1 = x1.read_f64::<NetworkEndian>()? as f32;
        let y1 = y1.read_f64::<NetworkEndian>()? as f32;
        let x2 = x2.read_f64::<NetworkEndian>()? as f32;
        let y2 = y2.read_f64::<NetworkEndian>()? as f32;
        // let str = String::from_utf8(bytes.as_bytes().to_vec())?;
        // if let [x1, y1, x2, y2] = str.split(',').collect::<Vec<_>>()[..] {
        //     let x1 = x1.trim_start_matches(['(', ' ', '\'']).parse()?;
        //     let y1 = y1.trim_start_matches([')', ' ']).parse()?;
        //     let x2 = x2.trim_start_matches(['(', ' ']).parse()?;
        //     let y2 = y2.trim_start_matches([')', ' ', '\'']).parse()?;
        //     Ok(Box { point1: Point { x: x1, y: y1 }, point2: Point { x: x2, y: y2 } })
        // } else {
        //     Err(anyhow::anyhow!("Unable to parse Postgresql returned value as Box").into())
        // }
        Ok(Box { point1: Point { x: x1, y: y1 }, point2: Point { x: x2, y: y2 } })
    }
}
// impl ToSql<sql_types::Box, Pg> for Box {
//     fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> ResultSer {
//         out.write_all(
//             format!(
//                 "( {} , {} ) , ( {} , {} )",
//                 self.point1.x,
//                 self.point1.y,
//                 self.point2.x,
//                 self.point2.y
//             ).as_bytes()
//         )?;
//         Ok(diesel::serialize::IsNull::No)
//     }
// }
impl Queryable<sql_types::Box, Pg> for Box {
    type Row = Box;

    fn build(row: Self::Row) -> ResultDe<Self> {
        Ok(row)
    }
}
impl Expression for Box {
    type SqlType = sql_types::Box;
}
impl<QS> AppearsOnTable<QS> for Box {}
impl QueryFragment<Pg> for Box {
    fn walk_ast<'b>(
        &'b self,
        mut pass: diesel::query_builder::AstPass<'_, 'b, Pg>
    ) -> diesel::prelude::QueryResult<()> {
        pass.push_sql(
            &format!(
                "'( {} , {} ) , ( {} , {} )'",
                self.point1.x,
                self.point1.y,
                self.point2.x,
                self.point2.y
            )
        );
        Ok(())
    }
}
impl Box {
    // The first two functions are copied from example. I guess `geo` can be introduced.

    pub fn intersection_area(&self, another: &Box) -> f32 {
        (self.point2.x.min(another.point2.x) - self.point1.x.max(another.point1.x)) *
            (self.point2.y.min(another.point2.y) - self.point1.y.max(another.point1.y))
    }

    pub fn union_area(self, another: &Box) -> f32 {
        self.area() + another.area() - self.intersection_area(another)
    }

    pub fn height(self) -> f32 {
        self.point2.y - self.point1.y
    }

    pub fn width(self) -> f32 {
        self.point2.x - self.point1.x
    }

    pub fn area(self) -> f32 {
        self.height() * self.width()
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
