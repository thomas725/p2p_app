use diesel::Insertable;
use crate::schema::identities;


#[derive(Insertable, Debug)]
#[diesel(table_name = identities)]
pub struct NewIdentity {
    pub key: Vec<u8>,
}