// This file was generated with `clorinde`. Do not modify.

#[derive(Debug)]
pub struct CreateTenantParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub id: uuid::Uuid,
    pub name: T1,
    pub slug: T2,
}
#[derive(Debug)]
pub struct UpdateTenantParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub name: T1,
    pub slug: T2,
    pub id: uuid::Uuid,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Tenant {
    pub id: uuid::Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}
pub struct TenantBorrowed<'a> {
    pub id: uuid::Uuid,
    pub name: &'a str,
    pub slug: &'a str,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}
impl<'a> From<TenantBorrowed<'a>> for Tenant {
    fn from(
        TenantBorrowed {
            id,
            name,
            slug,
            created_at,
            updated_at,
        }: TenantBorrowed<'a>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            slug: slug.into(),
            created_at,
            updated_at,
        }
    }
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct TenantQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<TenantBorrowed, tokio_postgres::Error>,
    mapper: fn(TenantBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> TenantQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(self, mapper: fn(TenantBorrowed) -> R) -> TenantQuery<'c, 'a, 's, C, R, N> {
        TenantQuery {
            client: self.client,
            params: self.params,
            query: self.query,
            cached: self.cached,
            extractor: self.extractor,
            mapper,
        }
    }
    pub async fn one(self) -> Result<T, tokio_postgres::Error> {
        let row =
            crate::client::async_::one(self.client, self.query, &self.params, self.cached).await?;
        Ok((self.mapper)((self.extractor)(&row)?))
    }
    pub async fn all(self) -> Result<Vec<T>, tokio_postgres::Error> {
        self.iter().await?.try_collect().await
    }
    pub async fn opt(self) -> Result<Option<T>, tokio_postgres::Error> {
        let opt_row =
            crate::client::async_::opt(self.client, self.query, &self.params, self.cached).await?;
        Ok(opt_row
            .map(|row| {
                let extracted = (self.extractor)(&row)?;
                Ok((self.mapper)(extracted))
            })
            .transpose()?)
    }
    pub async fn iter(
        self,
    ) -> Result<
        impl futures::Stream<Item = Result<T, tokio_postgres::Error>> + 'c,
        tokio_postgres::Error,
    > {
        let stream = crate::client::async_::raw(
            self.client,
            self.query,
            crate::slice_iter(&self.params),
            self.cached,
        )
        .await?;
        let mapped = stream
            .map(move |res| {
                res.and_then(|row| {
                    let extracted = (self.extractor)(&row)?;
                    Ok((self.mapper)(extracted))
                })
            })
            .into_stream();
        Ok(mapped)
    }
}
pub struct GetTenantByIdStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_tenant_by_id() -> GetTenantByIdStmt {
    GetTenantByIdStmt(
        "SELECT id, name, slug, created_at, updated_at FROM tenants WHERE id = $1",
        None,
    )
}
impl GetTenantByIdStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient>(
        &'s self,
        client: &'c C,
        id: &'a uuid::Uuid,
    ) -> TenantQuery<'c, 'a, 's, C, Tenant, 1> {
        TenantQuery {
            client,
            params: [id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<TenantBorrowed, tokio_postgres::Error> {
                    Ok(TenantBorrowed {
                        id: row.try_get(0)?,
                        name: row.try_get(1)?,
                        slug: row.try_get(2)?,
                        created_at: row.try_get(3)?,
                        updated_at: row.try_get(4)?,
                    })
                },
            mapper: |it| Tenant::from(it),
        }
    }
}
pub struct GetTenantBySlugStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_tenant_by_slug() -> GetTenantBySlugStmt {
    GetTenantBySlugStmt(
        "SELECT id, name, slug, created_at, updated_at FROM tenants WHERE slug = $1",
        None,
    )
}
impl GetTenantBySlugStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>(
        &'s self,
        client: &'c C,
        slug: &'a T1,
    ) -> TenantQuery<'c, 'a, 's, C, Tenant, 1> {
        TenantQuery {
            client,
            params: [slug],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<TenantBorrowed, tokio_postgres::Error> {
                    Ok(TenantBorrowed {
                        id: row.try_get(0)?,
                        name: row.try_get(1)?,
                        slug: row.try_get(2)?,
                        created_at: row.try_get(3)?,
                        updated_at: row.try_get(4)?,
                    })
                },
            mapper: |it| Tenant::from(it),
        }
    }
}
pub struct ListTenantsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_tenants() -> ListTenantsStmt {
    ListTenantsStmt(
        "SELECT id, name, slug, created_at, updated_at FROM tenants ORDER BY name",
        None,
    )
}
impl ListTenantsStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient>(
        &'s self,
        client: &'c C,
    ) -> TenantQuery<'c, 'a, 's, C, Tenant, 0> {
        TenantQuery {
            client,
            params: [],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<TenantBorrowed, tokio_postgres::Error> {
                    Ok(TenantBorrowed {
                        id: row.try_get(0)?,
                        name: row.try_get(1)?,
                        slug: row.try_get(2)?,
                        created_at: row.try_get(3)?,
                        updated_at: row.try_get(4)?,
                    })
                },
            mapper: |it| Tenant::from(it),
        }
    }
}
pub struct CreateTenantStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn create_tenant() -> CreateTenantStmt {
    CreateTenantStmt(
        "INSERT INTO tenants (id, name, slug, created_at, updated_at) VALUES ($1, $2, $3, NOW(), NOW()) RETURNING id, name, slug, created_at, updated_at",
        None,
    )
}
impl CreateTenantStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        id: &'a uuid::Uuid,
        name: &'a T1,
        slug: &'a T2,
    ) -> TenantQuery<'c, 'a, 's, C, Tenant, 3> {
        TenantQuery {
            client,
            params: [id, name, slug],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<TenantBorrowed, tokio_postgres::Error> {
                    Ok(TenantBorrowed {
                        id: row.try_get(0)?,
                        name: row.try_get(1)?,
                        slug: row.try_get(2)?,
                        created_at: row.try_get(3)?,
                        updated_at: row.try_get(4)?,
                    })
                },
            mapper: |it| Tenant::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        CreateTenantParams<T1, T2>,
        TenantQuery<'c, 'a, 's, C, Tenant, 3>,
        C,
    > for CreateTenantStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a CreateTenantParams<T1, T2>,
    ) -> TenantQuery<'c, 'a, 's, C, Tenant, 3> {
        self.bind(client, &params.id, &params.name, &params.slug)
    }
}
pub struct UpdateTenantStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn update_tenant() -> UpdateTenantStmt {
    UpdateTenantStmt(
        "UPDATE tenants SET name = $1, slug = $2, updated_at = NOW() WHERE id = $3 RETURNING id, name, slug, created_at, updated_at",
        None,
    )
}
impl UpdateTenantStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        name: &'a T1,
        slug: &'a T2,
        id: &'a uuid::Uuid,
    ) -> TenantQuery<'c, 'a, 's, C, Tenant, 3> {
        TenantQuery {
            client,
            params: [name, slug, id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<TenantBorrowed, tokio_postgres::Error> {
                    Ok(TenantBorrowed {
                        id: row.try_get(0)?,
                        name: row.try_get(1)?,
                        slug: row.try_get(2)?,
                        created_at: row.try_get(3)?,
                        updated_at: row.try_get(4)?,
                    })
                },
            mapper: |it| Tenant::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        UpdateTenantParams<T1, T2>,
        TenantQuery<'c, 'a, 's, C, Tenant, 3>,
        C,
    > for UpdateTenantStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a UpdateTenantParams<T1, T2>,
    ) -> TenantQuery<'c, 'a, 's, C, Tenant, 3> {
        self.bind(client, &params.name, &params.slug, &params.id)
    }
}
pub struct DeleteTenantStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn delete_tenant() -> DeleteTenantStmt {
    DeleteTenantStmt("DELETE FROM tenants WHERE id = $1", None)
}
impl DeleteTenantStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient>(
        &'s self,
        client: &'c C,
        id: &'a uuid::Uuid,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[id]).await
    }
}
