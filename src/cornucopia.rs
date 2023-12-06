// This file was generated with `cornucopia`. Do not modify.

#[allow(clippy :: all, clippy :: pedantic)] #[allow(unused_variables)]
#[allow(unused_imports)] #[allow(dead_code)] pub mod types { }#[allow(clippy :: all, clippy :: pedantic)] #[allow(unused_variables)]
#[allow(unused_imports)] #[allow(dead_code)] pub mod queries
{ pub mod newsletters
{ use futures::{{StreamExt, TryStreamExt}};use futures; use cornucopia_async::GenericClient;pub struct StringQuery < 'a, C : GenericClient, T, const N : usize >
{
    client : & 'a  C, params :
    [& 'a (dyn postgres_types :: ToSql + Sync) ; N], stmt : & 'a mut cornucopia_async
    :: private :: Stmt, extractor : fn(& tokio_postgres :: Row) -> & str,
    mapper : fn(& str) -> T,
} impl < 'a, C, T : 'a, const N : usize > StringQuery < 'a, C, T, N >
where C : GenericClient
{
    pub fn map < R > (self, mapper : fn(& str) -> R) -> StringQuery
    < 'a, C, R, N >
    {
        StringQuery
        {
            client : self.client, params : self.params, stmt : self.stmt,
            extractor : self.extractor, mapper,
        }
    } pub async fn one(self) -> Result < T, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ; let row =
        self.client.query_one(stmt, & self.params) .await ? ;
        Ok((self.mapper) ((self.extractor) (& row)))
    } pub async fn all(self) -> Result < Vec < T >, tokio_postgres :: Error >
    { self.iter() .await ?.try_collect().await } pub async fn opt(self) -> Result
    < Option < T >, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ;
        Ok(self.client.query_opt(stmt, & self.params) .await
        ?.map(| row | (self.mapper) ((self.extractor) (& row))))
    } pub async fn iter(self,) -> Result < impl futures::Stream < Item = Result
    < T, tokio_postgres :: Error >> + 'a, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ; let it =
        self.client.query_raw(stmt, cornucopia_async :: private ::
        slice_iter(& self.params)) .await ?
        .map(move | res |
        res.map(| row | (self.mapper) ((self.extractor) (& row)))) .into_stream() ;
        Ok(it)
    }
}pub fn query_confirmed_subscribers() -> QueryConfirmedSubscribersStmt
{ QueryConfirmedSubscribersStmt(cornucopia_async :: private :: Stmt :: new("SELECT email
FROM subscriptions
WHERE status = 'confirmed'")) } pub
struct QueryConfirmedSubscribersStmt(cornucopia_async :: private :: Stmt) ; impl
QueryConfirmedSubscribersStmt { pub fn bind < 'a, C : GenericClient, >
(& 'a mut self, client : & 'a  C,
) -> StringQuery < 'a, C,
String, 0 >
{
    StringQuery
    {
        client, params : [], stmt : & mut self.0, extractor :
        | row | { row.get(0) }, mapper : | it | { it.into() },
    }
} }}pub mod subscriptions
{ use futures::{{StreamExt, TryStreamExt}};use futures; use cornucopia_async::GenericClient;#[derive( Debug)] pub struct InsertNewSubscriptionParams < T1 : cornucopia_async::StringSql,T2 : cornucopia_async::StringSql,> { pub id : uuid::Uuid,pub email : T1,pub name : T2,pub subscribed_at : time::OffsetDateTime,}#[derive( Debug)] pub struct InsertNewTokenParams < T1 : cornucopia_async::StringSql,> { pub subscription_token : T1,pub subscriber_id : uuid::Uuid,}#[derive( Debug)] pub struct InsertTokenByEmailParams < T1 : cornucopia_async::StringSql,T2 : cornucopia_async::StringSql,> { pub token : T1,pub email : T2,}pub struct StringQuery < 'a, C : GenericClient, T, const N : usize >
{
    client : & 'a  C, params :
    [& 'a (dyn postgres_types :: ToSql + Sync) ; N], stmt : & 'a mut cornucopia_async
    :: private :: Stmt, extractor : fn(& tokio_postgres :: Row) -> & str,
    mapper : fn(& str) -> T,
} impl < 'a, C, T : 'a, const N : usize > StringQuery < 'a, C, T, N >
where C : GenericClient
{
    pub fn map < R > (self, mapper : fn(& str) -> R) -> StringQuery
    < 'a, C, R, N >
    {
        StringQuery
        {
            client : self.client, params : self.params, stmt : self.stmt,
            extractor : self.extractor, mapper,
        }
    } pub async fn one(self) -> Result < T, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ; let row =
        self.client.query_one(stmt, & self.params) .await ? ;
        Ok((self.mapper) ((self.extractor) (& row)))
    } pub async fn all(self) -> Result < Vec < T >, tokio_postgres :: Error >
    { self.iter() .await ?.try_collect().await } pub async fn opt(self) -> Result
    < Option < T >, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ;
        Ok(self.client.query_opt(stmt, & self.params) .await
        ?.map(| row | (self.mapper) ((self.extractor) (& row))))
    } pub async fn iter(self,) -> Result < impl futures::Stream < Item = Result
    < T, tokio_postgres :: Error >> + 'a, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ; let it =
        self.client.query_raw(stmt, cornucopia_async :: private ::
        slice_iter(& self.params)) .await ?
        .map(move | res |
        res.map(| row | (self.mapper) ((self.extractor) (& row)))) .into_stream() ;
        Ok(it)
    }
}pub struct UuidUuidQuery < 'a, C : GenericClient, T, const N : usize >
{
    client : & 'a  C, params :
    [& 'a (dyn postgres_types :: ToSql + Sync) ; N], stmt : & 'a mut cornucopia_async
    :: private :: Stmt, extractor : fn(& tokio_postgres :: Row) -> uuid::Uuid,
    mapper : fn(uuid::Uuid) -> T,
} impl < 'a, C, T : 'a, const N : usize > UuidUuidQuery < 'a, C, T, N >
where C : GenericClient
{
    pub fn map < R > (self, mapper : fn(uuid::Uuid) -> R) -> UuidUuidQuery
    < 'a, C, R, N >
    {
        UuidUuidQuery
        {
            client : self.client, params : self.params, stmt : self.stmt,
            extractor : self.extractor, mapper,
        }
    } pub async fn one(self) -> Result < T, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ; let row =
        self.client.query_one(stmt, & self.params) .await ? ;
        Ok((self.mapper) ((self.extractor) (& row)))
    } pub async fn all(self) -> Result < Vec < T >, tokio_postgres :: Error >
    { self.iter() .await ?.try_collect().await } pub async fn opt(self) -> Result
    < Option < T >, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ;
        Ok(self.client.query_opt(stmt, & self.params) .await
        ?.map(| row | (self.mapper) ((self.extractor) (& row))))
    } pub async fn iter(self,) -> Result < impl futures::Stream < Item = Result
    < T, tokio_postgres :: Error >> + 'a, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ; let it =
        self.client.query_raw(stmt, cornucopia_async :: private ::
        slice_iter(& self.params)) .await ?
        .map(move | res |
        res.map(| row | (self.mapper) ((self.extractor) (& row)))) .into_stream() ;
        Ok(it)
    }
}pub struct I32Query < 'a, C : GenericClient, T, const N : usize >
{
    client : & 'a  C, params :
    [& 'a (dyn postgres_types :: ToSql + Sync) ; N], stmt : & 'a mut cornucopia_async
    :: private :: Stmt, extractor : fn(& tokio_postgres :: Row) -> i32,
    mapper : fn(i32) -> T,
} impl < 'a, C, T : 'a, const N : usize > I32Query < 'a, C, T, N >
where C : GenericClient
{
    pub fn map < R > (self, mapper : fn(i32) -> R) -> I32Query
    < 'a, C, R, N >
    {
        I32Query
        {
            client : self.client, params : self.params, stmt : self.stmt,
            extractor : self.extractor, mapper,
        }
    } pub async fn one(self) -> Result < T, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ; let row =
        self.client.query_one(stmt, & self.params) .await ? ;
        Ok((self.mapper) ((self.extractor) (& row)))
    } pub async fn all(self) -> Result < Vec < T >, tokio_postgres :: Error >
    { self.iter() .await ?.try_collect().await } pub async fn opt(self) -> Result
    < Option < T >, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ;
        Ok(self.client.query_opt(stmt, & self.params) .await
        ?.map(| row | (self.mapper) ((self.extractor) (& row))))
    } pub async fn iter(self,) -> Result < impl futures::Stream < Item = Result
    < T, tokio_postgres :: Error >> + 'a, tokio_postgres :: Error >
    {
        let stmt = self.stmt.prepare(self.client) .await ? ; let it =
        self.client.query_raw(stmt, cornucopia_async :: private ::
        slice_iter(& self.params)) .await ?
        .map(move | res |
        res.map(| row | (self.mapper) ((self.extractor) (& row)))) .into_stream() ;
        Ok(it)
    }
}pub fn insert_new_subscription() -> InsertNewSubscriptionStmt
{ InsertNewSubscriptionStmt(cornucopia_async :: private :: Stmt :: new("INSERT INTO subscriptions(id, email, name, subscribed_at, status)
VALUES ($1, $2, $3, $4, 'pending_confirmation')")) } pub
struct InsertNewSubscriptionStmt(cornucopia_async :: private :: Stmt) ; impl
InsertNewSubscriptionStmt { pub async fn bind < 'a, C : GenericClient, T1 : cornucopia_async::StringSql,T2 : cornucopia_async::StringSql,>
(& 'a mut self, client : & 'a  C,
id : & 'a uuid::Uuid,email : & 'a T1,name : & 'a T2,subscribed_at : & 'a time::OffsetDateTime,) -> Result < u64, tokio_postgres :: Error >
{
    let stmt = self.0.prepare(client) .await ? ;
    client.execute(stmt, & [id,email,name,subscribed_at,]) .await
} }impl < 'a, C : GenericClient + Send + Sync, T1 : cornucopia_async::StringSql,T2 : cornucopia_async::StringSql,>
cornucopia_async :: Params < 'a, InsertNewSubscriptionParams < T1,T2,>, std::pin::Pin<Box<dyn futures::Future<Output = Result <
u64, tokio_postgres :: Error > > + Send + 'a>>, C > for InsertNewSubscriptionStmt
{
    fn
    params(& 'a mut self, client : & 'a  C, params : & 'a
    InsertNewSubscriptionParams < T1,T2,>) -> std::pin::Pin<Box<dyn futures::Future<Output = Result < u64, tokio_postgres ::
    Error > > + Send + 'a>> { Box::pin(self.bind(client, & params.id,& params.email,& params.name,& params.subscribed_at,) ) }
}pub fn insert_new_token() -> InsertNewTokenStmt
{ InsertNewTokenStmt(cornucopia_async :: private :: Stmt :: new("INSERT INTO subscription_tokens(subscription_token, subscriber_id)
VALUES ($1, $2)")) } pub
struct InsertNewTokenStmt(cornucopia_async :: private :: Stmt) ; impl
InsertNewTokenStmt { pub async fn bind < 'a, C : GenericClient, T1 : cornucopia_async::StringSql,>
(& 'a mut self, client : & 'a  C,
subscription_token : & 'a T1,subscriber_id : & 'a uuid::Uuid,) -> Result < u64, tokio_postgres :: Error >
{
    let stmt = self.0.prepare(client) .await ? ;
    client.execute(stmt, & [subscription_token,subscriber_id,]) .await
} }impl < 'a, C : GenericClient + Send + Sync, T1 : cornucopia_async::StringSql,>
cornucopia_async :: Params < 'a, InsertNewTokenParams < T1,>, std::pin::Pin<Box<dyn futures::Future<Output = Result <
u64, tokio_postgres :: Error > > + Send + 'a>>, C > for InsertNewTokenStmt
{
    fn
    params(& 'a mut self, client : & 'a  C, params : & 'a
    InsertNewTokenParams < T1,>) -> std::pin::Pin<Box<dyn futures::Future<Output = Result < u64, tokio_postgres ::
    Error > > + Send + 'a>> { Box::pin(self.bind(client, & params.subscription_token,& params.subscriber_id,) ) }
}pub fn delete_token_by_email() -> DeleteTokenByEmailStmt
{ DeleteTokenByEmailStmt(cornucopia_async :: private :: Stmt :: new("DELETE FROM subscription_tokens
    WHERE subscriber_id = (
    SELECT id
    FROM subscriptions
    WHERE email = $1
)")) } pub
struct DeleteTokenByEmailStmt(cornucopia_async :: private :: Stmt) ; impl
DeleteTokenByEmailStmt { pub async fn bind < 'a, C : GenericClient, T1 : cornucopia_async::StringSql,>
(& 'a mut self, client : & 'a  C,
email : & 'a T1,) -> Result < u64, tokio_postgres :: Error >
{
    let stmt = self.0.prepare(client) .await ? ;
    client.execute(stmt, & [email,]) .await
} }pub fn insert_token_by_email() -> InsertTokenByEmailStmt
{ InsertTokenByEmailStmt(cornucopia_async :: private :: Stmt :: new("INSERT INTO subscription_tokens
(subscription_token, subscriber_id)
VALUES ($1, (
        SELECT id
        FROM subscriptions
        WHERE email = $2
    )
)")) } pub
struct InsertTokenByEmailStmt(cornucopia_async :: private :: Stmt) ; impl
InsertTokenByEmailStmt { pub async fn bind < 'a, C : GenericClient, T1 : cornucopia_async::StringSql,T2 : cornucopia_async::StringSql,>
(& 'a mut self, client : & 'a  C,
token : & 'a T1,email : & 'a T2,) -> Result < u64, tokio_postgres :: Error >
{
    let stmt = self.0.prepare(client) .await ? ;
    client.execute(stmt, & [token,email,]) .await
} }impl < 'a, C : GenericClient + Send + Sync, T1 : cornucopia_async::StringSql,T2 : cornucopia_async::StringSql,>
cornucopia_async :: Params < 'a, InsertTokenByEmailParams < T1,T2,>, std::pin::Pin<Box<dyn futures::Future<Output = Result <
u64, tokio_postgres :: Error > > + Send + 'a>>, C > for InsertTokenByEmailStmt
{
    fn
    params(& 'a mut self, client : & 'a  C, params : & 'a
    InsertTokenByEmailParams < T1,T2,>) -> std::pin::Pin<Box<dyn futures::Future<Output = Result < u64, tokio_postgres ::
    Error > > + Send + 'a>> { Box::pin(self.bind(client, & params.token,& params.email,) ) }
}pub fn get_status() -> GetStatusStmt
{ GetStatusStmt(cornucopia_async :: private :: Stmt :: new("SELECT status
FROM subscriptions
WHERE email = $1")) } pub
struct GetStatusStmt(cornucopia_async :: private :: Stmt) ; impl
GetStatusStmt { pub fn bind < 'a, C : GenericClient, T1 : cornucopia_async::StringSql,>
(& 'a mut self, client : & 'a  C,
email : & 'a T1,) -> StringQuery < 'a, C,
String, 1 >
{
    StringQuery
    {
        client, params : [email,], stmt : & mut self.0, extractor :
        | row | { row.get(0) }, mapper : | it | { it.into() },
    }
} }pub fn confirm_subscriber() -> ConfirmSubscriberStmt
{ ConfirmSubscriberStmt(cornucopia_async :: private :: Stmt :: new("UPDATE subscriptions SET status = 'confirmed' WHERE id = $1 AND status = 'pending_confirmation'")) } pub
struct ConfirmSubscriberStmt(cornucopia_async :: private :: Stmt) ; impl
ConfirmSubscriberStmt { pub async fn bind < 'a, C : GenericClient, >
(& 'a mut self, client : & 'a  C,
sub_id : & 'a uuid::Uuid,) -> Result < u64, tokio_postgres :: Error >
{
    let stmt = self.0.prepare(client) .await ? ;
    client.execute(stmt, & [sub_id,]) .await
} }pub fn get_subscriber_id_from_token() -> GetSubscriberIdFromTokenStmt
{ GetSubscriberIdFromTokenStmt(cornucopia_async :: private :: Stmt :: new("SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1")) } pub
struct GetSubscriberIdFromTokenStmt(cornucopia_async :: private :: Stmt) ; impl
GetSubscriberIdFromTokenStmt { pub fn bind < 'a, C : GenericClient, T1 : cornucopia_async::StringSql,>
(& 'a mut self, client : & 'a  C,
sub_token : & 'a T1,) -> UuidUuidQuery < 'a, C,
uuid::Uuid, 1 >
{
    UuidUuidQuery
    {
        client, params : [sub_token,], stmt : & mut self.0, extractor :
        | row | { row.get(0) }, mapper : | it | { it },
    }
} }pub fn get_countdown() -> GetCountdownStmt
{ GetCountdownStmt(cornucopia_async :: private :: Stmt :: new("WITH RECURSIVE countdown(val) AS (
    SELECT 10 AS val -- initial, non-recursive query
    UNION -- every recursive CTE needs `UNION` keyword
    SELECT val - 1 FROM countdown WHERE val > 1 -- recursive query
)
SELECT *
FROM countdown")) } pub
struct GetCountdownStmt(cornucopia_async :: private :: Stmt) ; impl
GetCountdownStmt { pub fn bind < 'a, C : GenericClient, >
(& 'a mut self, client : & 'a  C,
) -> I32Query < 'a, C,
i32, 0 >
{
    I32Query
    {
        client, params : [], stmt : & mut self.0, extractor :
        | row | { row.get(0) }, mapper : | it | { it },
    }
} }}}