// This file was generated with `clorinde`. Do not modify.

#[derive(Debug)]
pub struct CreatePipelineParams<T1: crate::StringSql, T2: crate::StringSql, T3: crate::JsonSql> {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub name: T1,
    pub repository: T2,
    pub config: T3,
}
#[derive(Debug)]
pub struct UpdatePipelineConfigParams<T1: crate::JsonSql> {
    pub config: T1,
    pub id: uuid::Uuid,
}
#[derive(Clone, Copy, Debug)]
pub struct ListRunsByPipelineParams {
    pub pipeline_id: uuid::Uuid,
    pub limit: i64,
}
#[derive(Debug)]
pub struct CreateRunParams<T1: crate::JsonSql, T2: crate::JsonSql> {
    pub id: uuid::Uuid,
    pub pipeline_id: uuid::Uuid,
    pub trigger_info: T1,
    pub git_info: T2,
}
#[derive(Debug)]
pub struct UpdateRunStatusParams<T1: crate::StringSql> {
    pub status: T1,
    pub id: uuid::Uuid,
}
#[derive(Debug)]
pub struct MarkRunFinishedParams<T1: crate::StringSql> {
    pub status: T1,
    pub id: uuid::Uuid,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Pipeline {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub name: String,
    pub repository: String,
    pub config: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}
pub struct PipelineBorrowed<'a> {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub name: &'a str,
    pub repository: &'a str,
    pub config: postgres_types::Json<&'a serde_json::value::RawValue>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub updated_at: chrono::DateTime<chrono::FixedOffset>,
}
impl<'a> From<PipelineBorrowed<'a>> for Pipeline {
    fn from(
        PipelineBorrowed {
            id,
            tenant_id,
            name,
            repository,
            config,
            created_at,
            updated_at,
        }: PipelineBorrowed<'a>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            name: name.into(),
            repository: repository.into(),
            config: serde_json::from_str(config.0.get()).unwrap(),
            created_at,
            updated_at,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineRun {
    pub id: uuid::Uuid,
    pub pipeline_id: uuid::Uuid,
    pub number: i64,
    pub status: String,
    pub trigger_info: serde_json::Value,
    pub git_info: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub started_at: chrono::DateTime<chrono::FixedOffset>,
    pub finished_at: chrono::DateTime<chrono::FixedOffset>,
}
pub struct PipelineRunBorrowed<'a> {
    pub id: uuid::Uuid,
    pub pipeline_id: uuid::Uuid,
    pub number: i64,
    pub status: &'a str,
    pub trigger_info: postgres_types::Json<&'a serde_json::value::RawValue>,
    pub git_info: postgres_types::Json<&'a serde_json::value::RawValue>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub started_at: chrono::DateTime<chrono::FixedOffset>,
    pub finished_at: chrono::DateTime<chrono::FixedOffset>,
}
impl<'a> From<PipelineRunBorrowed<'a>> for PipelineRun {
    fn from(
        PipelineRunBorrowed {
            id,
            pipeline_id,
            number,
            status,
            trigger_info,
            git_info,
            created_at,
            started_at,
            finished_at,
        }: PipelineRunBorrowed<'a>,
    ) -> Self {
        Self {
            id,
            pipeline_id,
            number,
            status: status.into(),
            trigger_info: serde_json::from_str(trigger_info.0.get()).unwrap(),
            git_info: serde_json::from_str(git_info.0.get()).unwrap(),
            created_at,
            started_at,
            finished_at,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct NextRunNumber {
    pub next_number: i64,
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct PipelineQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<PipelineBorrowed, tokio_postgres::Error>,
    mapper: fn(PipelineBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> PipelineQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(self, mapper: fn(PipelineBorrowed) -> R) -> PipelineQuery<'c, 'a, 's, C, R, N> {
        PipelineQuery {
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
pub struct PipelineRunQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<PipelineRunBorrowed, tokio_postgres::Error>,
    mapper: fn(PipelineRunBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> PipelineRunQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(PipelineRunBorrowed) -> R,
    ) -> PipelineRunQuery<'c, 'a, 's, C, R, N> {
        PipelineRunQuery {
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
pub struct NextRunNumberQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<NextRunNumber, tokio_postgres::Error>,
    mapper: fn(NextRunNumber) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> NextRunNumberQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(self, mapper: fn(NextRunNumber) -> R) -> NextRunNumberQuery<'c, 'a, 's, C, R, N> {
        NextRunNumberQuery {
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
pub struct GetPipelineByIdStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_pipeline_by_id() -> GetPipelineByIdStmt {
    GetPipelineByIdStmt(
        "SELECT id, tenant_id, name, repository, config, created_at, updated_at FROM pipelines WHERE id = $1",
        None,
    )
}
impl GetPipelineByIdStmt {
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
    ) -> PipelineQuery<'c, 'a, 's, C, Pipeline, 1> {
        PipelineQuery {
            client,
            params: [id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<PipelineBorrowed, tokio_postgres::Error> {
                    Ok(PipelineBorrowed {
                        id: row.try_get(0)?,
                        tenant_id: row.try_get(1)?,
                        name: row.try_get(2)?,
                        repository: row.try_get(3)?,
                        config: row.try_get(4)?,
                        created_at: row.try_get(5)?,
                        updated_at: row.try_get(6)?,
                    })
                },
            mapper: |it| Pipeline::from(it),
        }
    }
}
pub struct ListPipelinesByTenantStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_pipelines_by_tenant() -> ListPipelinesByTenantStmt {
    ListPipelinesByTenantStmt(
        "SELECT id, tenant_id, name, repository, config, created_at, updated_at FROM pipelines WHERE tenant_id = $1 ORDER BY name",
        None,
    )
}
impl ListPipelinesByTenantStmt {
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
        tenant_id: &'a uuid::Uuid,
    ) -> PipelineQuery<'c, 'a, 's, C, Pipeline, 1> {
        PipelineQuery {
            client,
            params: [tenant_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<PipelineBorrowed, tokio_postgres::Error> {
                    Ok(PipelineBorrowed {
                        id: row.try_get(0)?,
                        tenant_id: row.try_get(1)?,
                        name: row.try_get(2)?,
                        repository: row.try_get(3)?,
                        config: row.try_get(4)?,
                        created_at: row.try_get(5)?,
                        updated_at: row.try_get(6)?,
                    })
                },
            mapper: |it| Pipeline::from(it),
        }
    }
}
pub struct CreatePipelineStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn create_pipeline() -> CreatePipelineStmt {
    CreatePipelineStmt(
        "INSERT INTO pipelines (id, tenant_id, name, repository, config, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, NOW(), NOW()) RETURNING id, tenant_id, name, repository, config, created_at, updated_at",
        None,
    )
}
impl CreatePipelineStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<
        'c,
        'a,
        's,
        C: GenericClient,
        T1: crate::StringSql,
        T2: crate::StringSql,
        T3: crate::JsonSql,
    >(
        &'s self,
        client: &'c C,
        id: &'a uuid::Uuid,
        tenant_id: &'a uuid::Uuid,
        name: &'a T1,
        repository: &'a T2,
        config: &'a T3,
    ) -> PipelineQuery<'c, 'a, 's, C, Pipeline, 5> {
        PipelineQuery {
            client,
            params: [id, tenant_id, name, repository, config],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<PipelineBorrowed, tokio_postgres::Error> {
                    Ok(PipelineBorrowed {
                        id: row.try_get(0)?,
                        tenant_id: row.try_get(1)?,
                        name: row.try_get(2)?,
                        repository: row.try_get(3)?,
                        config: row.try_get(4)?,
                        created_at: row.try_get(5)?,
                        updated_at: row.try_get(6)?,
                    })
                },
            mapper: |it| Pipeline::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql, T3: crate::JsonSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        CreatePipelineParams<T1, T2, T3>,
        PipelineQuery<'c, 'a, 's, C, Pipeline, 5>,
        C,
    > for CreatePipelineStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a CreatePipelineParams<T1, T2, T3>,
    ) -> PipelineQuery<'c, 'a, 's, C, Pipeline, 5> {
        self.bind(
            client,
            &params.id,
            &params.tenant_id,
            &params.name,
            &params.repository,
            &params.config,
        )
    }
}
pub struct UpdatePipelineConfigStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn update_pipeline_config() -> UpdatePipelineConfigStmt {
    UpdatePipelineConfigStmt(
        "UPDATE pipelines SET config = $1, updated_at = NOW() WHERE id = $2 RETURNING id, tenant_id, name, repository, config, created_at, updated_at",
        None,
    )
}
impl UpdatePipelineConfigStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::JsonSql>(
        &'s self,
        client: &'c C,
        config: &'a T1,
        id: &'a uuid::Uuid,
    ) -> PipelineQuery<'c, 'a, 's, C, Pipeline, 2> {
        PipelineQuery {
            client,
            params: [config, id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<PipelineBorrowed, tokio_postgres::Error> {
                    Ok(PipelineBorrowed {
                        id: row.try_get(0)?,
                        tenant_id: row.try_get(1)?,
                        name: row.try_get(2)?,
                        repository: row.try_get(3)?,
                        config: row.try_get(4)?,
                        created_at: row.try_get(5)?,
                        updated_at: row.try_get(6)?,
                    })
                },
            mapper: |it| Pipeline::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::JsonSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        UpdatePipelineConfigParams<T1>,
        PipelineQuery<'c, 'a, 's, C, Pipeline, 2>,
        C,
    > for UpdatePipelineConfigStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a UpdatePipelineConfigParams<T1>,
    ) -> PipelineQuery<'c, 'a, 's, C, Pipeline, 2> {
        self.bind(client, &params.config, &params.id)
    }
}
pub struct DeletePipelineStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn delete_pipeline() -> DeletePipelineStmt {
    DeletePipelineStmt("DELETE FROM pipelines WHERE id = $1", None)
}
impl DeletePipelineStmt {
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
pub struct GetRunByIdStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_run_by_id() -> GetRunByIdStmt {
    GetRunByIdStmt(
        "SELECT id, pipeline_id, number, status, trigger_info, git_info, created_at, started_at, finished_at FROM pipeline_runs WHERE id = $1",
        None,
    )
}
impl GetRunByIdStmt {
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
    ) -> PipelineRunQuery<'c, 'a, 's, C, PipelineRun, 1> {
        PipelineRunQuery {
            client,
            params: [id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<PipelineRunBorrowed, tokio_postgres::Error> {
                    Ok(PipelineRunBorrowed {
                        id: row.try_get(0)?,
                        pipeline_id: row.try_get(1)?,
                        number: row.try_get(2)?,
                        status: row.try_get(3)?,
                        trigger_info: row.try_get(4)?,
                        git_info: row.try_get(5)?,
                        created_at: row.try_get(6)?,
                        started_at: row.try_get(7)?,
                        finished_at: row.try_get(8)?,
                    })
                },
            mapper: |it| PipelineRun::from(it),
        }
    }
}
pub struct ListRunsByPipelineStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_runs_by_pipeline() -> ListRunsByPipelineStmt {
    ListRunsByPipelineStmt(
        "SELECT id, pipeline_id, number, status, trigger_info, git_info, created_at, started_at, finished_at FROM pipeline_runs WHERE pipeline_id = $1 ORDER BY number DESC LIMIT $2",
        None,
    )
}
impl ListRunsByPipelineStmt {
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
        pipeline_id: &'a uuid::Uuid,
        limit: &'a i64,
    ) -> PipelineRunQuery<'c, 'a, 's, C, PipelineRun, 2> {
        PipelineRunQuery {
            client,
            params: [pipeline_id, limit],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<PipelineRunBorrowed, tokio_postgres::Error> {
                    Ok(PipelineRunBorrowed {
                        id: row.try_get(0)?,
                        pipeline_id: row.try_get(1)?,
                        number: row.try_get(2)?,
                        status: row.try_get(3)?,
                        trigger_info: row.try_get(4)?,
                        git_info: row.try_get(5)?,
                        created_at: row.try_get(6)?,
                        started_at: row.try_get(7)?,
                        finished_at: row.try_get(8)?,
                    })
                },
            mapper: |it| PipelineRun::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        ListRunsByPipelineParams,
        PipelineRunQuery<'c, 'a, 's, C, PipelineRun, 2>,
        C,
    > for ListRunsByPipelineStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a ListRunsByPipelineParams,
    ) -> PipelineRunQuery<'c, 'a, 's, C, PipelineRun, 2> {
        self.bind(client, &params.pipeline_id, &params.limit)
    }
}
pub struct CreateRunStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn create_run() -> CreateRunStmt {
    CreateRunStmt(
        "INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info, git_info, created_at) VALUES ( $1, $2, (SELECT COALESCE(MAX(number), 0) + 1 FROM pipeline_runs WHERE pipeline_id = $2), 'queued', $3, $4, NOW() ) RETURNING id, pipeline_id, number, status, trigger_info, git_info, created_at, started_at, finished_at",
        None,
    )
}
impl CreateRunStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::JsonSql, T2: crate::JsonSql>(
        &'s self,
        client: &'c C,
        id: &'a uuid::Uuid,
        pipeline_id: &'a uuid::Uuid,
        trigger_info: &'a T1,
        git_info: &'a T2,
    ) -> PipelineRunQuery<'c, 'a, 's, C, PipelineRun, 4> {
        PipelineRunQuery {
            client,
            params: [id, pipeline_id, trigger_info, git_info],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<PipelineRunBorrowed, tokio_postgres::Error> {
                    Ok(PipelineRunBorrowed {
                        id: row.try_get(0)?,
                        pipeline_id: row.try_get(1)?,
                        number: row.try_get(2)?,
                        status: row.try_get(3)?,
                        trigger_info: row.try_get(4)?,
                        git_info: row.try_get(5)?,
                        created_at: row.try_get(6)?,
                        started_at: row.try_get(7)?,
                        finished_at: row.try_get(8)?,
                    })
                },
            mapper: |it| PipelineRun::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::JsonSql, T2: crate::JsonSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        CreateRunParams<T1, T2>,
        PipelineRunQuery<'c, 'a, 's, C, PipelineRun, 4>,
        C,
    > for CreateRunStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a CreateRunParams<T1, T2>,
    ) -> PipelineRunQuery<'c, 'a, 's, C, PipelineRun, 4> {
        self.bind(
            client,
            &params.id,
            &params.pipeline_id,
            &params.trigger_info,
            &params.git_info,
        )
    }
}
pub struct UpdateRunStatusStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn update_run_status() -> UpdateRunStatusStmt {
    UpdateRunStatusStmt("UPDATE pipeline_runs SET status = $1 WHERE id = $2", None)
}
impl UpdateRunStatusStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>(
        &'s self,
        client: &'c C,
        status: &'a T1,
        id: &'a uuid::Uuid,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[status, id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpdateRunStatusParams<T1>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpdateRunStatusStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpdateRunStatusParams<T1>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.status, &params.id))
    }
}
pub struct MarkRunStartedStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn mark_run_started() -> MarkRunStartedStmt {
    MarkRunStartedStmt(
        "UPDATE pipeline_runs SET status = 'running', started_at = NOW() WHERE id = $1 AND started_at IS NULL",
        None,
    )
}
impl MarkRunStartedStmt {
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
pub struct MarkRunFinishedStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn mark_run_finished() -> MarkRunFinishedStmt {
    MarkRunFinishedStmt(
        "UPDATE pipeline_runs SET status = $1, finished_at = NOW() WHERE id = $2",
        None,
    )
}
impl MarkRunFinishedStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>(
        &'s self,
        client: &'c C,
        status: &'a T1,
        id: &'a uuid::Uuid,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[status, id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        MarkRunFinishedParams<T1>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for MarkRunFinishedStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a MarkRunFinishedParams<T1>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.status, &params.id))
    }
}
pub struct GetNextRunNumberStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_next_run_number() -> GetNextRunNumberStmt {
    GetNextRunNumberStmt(
        "SELECT COALESCE(MAX(number), 0) + 1 as next_number FROM pipeline_runs WHERE pipeline_id = $1",
        None,
    )
}
impl GetNextRunNumberStmt {
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
        pipeline_id: &'a uuid::Uuid,
    ) -> NextRunNumberQuery<'c, 'a, 's, C, NextRunNumber, 1> {
        NextRunNumberQuery {
            client,
            params: [pipeline_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row: &tokio_postgres::Row| -> Result<NextRunNumber, tokio_postgres::Error> {
                Ok(NextRunNumber {
                    next_number: row.try_get(0)?,
                })
            },
            mapper: |it| NextRunNumber::from(it),
        }
    }
}
