// This file was generated with `clorinde`. Do not modify.

#[derive(Debug)]
pub struct EnqueueJobParams<T1: crate::StringSql> {
    pub id: uuid::Uuid,
    pub pipeline_run_id: uuid::Uuid,
    pub stage_name: T1,
    pub priority: i32,
}
#[derive(Debug)]
pub struct FailJobParams<T1: crate::StringSql> {
    pub error: T1,
    pub id: uuid::Uuid,
}
#[derive(Debug)]
pub struct RetryJobParams<T1: crate::StringSql> {
    pub error: T1,
    pub id: uuid::Uuid,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub id: uuid::Uuid,
    pub pipeline_run_id: uuid::Uuid,
    pub stage_name: String,
    pub priority: i32,
    pub status: String,
    pub claimed_by: String,
    pub claimed_at: chrono::DateTime<chrono::FixedOffset>,
    pub error: String,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
}
pub struct JobBorrowed<'a> {
    pub id: uuid::Uuid,
    pub pipeline_run_id: uuid::Uuid,
    pub stage_name: &'a str,
    pub priority: i32,
    pub status: &'a str,
    pub claimed_by: &'a str,
    pub claimed_at: chrono::DateTime<chrono::FixedOffset>,
    pub error: &'a str,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
}
impl<'a> From<JobBorrowed<'a>> for Job {
    fn from(
        JobBorrowed {
            id,
            pipeline_run_id,
            stage_name,
            priority,
            status,
            claimed_by,
            claimed_at,
            error,
            created_at,
        }: JobBorrowed<'a>,
    ) -> Self {
        Self {
            id,
            pipeline_run_id,
            stage_name: stage_name.into(),
            priority,
            status: status.into(),
            claimed_by: claimed_by.into(),
            claimed_at,
            error: error.into(),
            created_at,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct PendingCount {
    pub count: i64,
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct JobQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<JobBorrowed, tokio_postgres::Error>,
    mapper: fn(JobBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> JobQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(self, mapper: fn(JobBorrowed) -> R) -> JobQuery<'c, 'a, 's, C, R, N> {
        JobQuery {
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
pub struct PendingCountQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<PendingCount, tokio_postgres::Error>,
    mapper: fn(PendingCount) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> PendingCountQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(self, mapper: fn(PendingCount) -> R) -> PendingCountQuery<'c, 'a, 's, C, R, N> {
        PendingCountQuery {
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
pub struct EnqueueJobStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn enqueue_job() -> EnqueueJobStmt {
    EnqueueJobStmt(
        "INSERT INTO job_queue (id, pipeline_run_id, stage_name, priority, status, created_at) VALUES ($1, $2, $3, $4, 'pending', NOW()) RETURNING id, pipeline_run_id, stage_name, priority, status, claimed_by, claimed_at, error, created_at",
        None,
    )
}
impl EnqueueJobStmt {
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
        id: &'a uuid::Uuid,
        pipeline_run_id: &'a uuid::Uuid,
        stage_name: &'a T1,
        priority: &'a i32,
    ) -> JobQuery<'c, 'a, 's, C, Job, 4> {
        JobQuery {
            client,
            params: [id, pipeline_run_id, stage_name, priority],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row: &tokio_postgres::Row| -> Result<JobBorrowed, tokio_postgres::Error> {
                Ok(JobBorrowed {
                    id: row.try_get(0)?,
                    pipeline_run_id: row.try_get(1)?,
                    stage_name: row.try_get(2)?,
                    priority: row.try_get(3)?,
                    status: row.try_get(4)?,
                    claimed_by: row.try_get(5)?,
                    claimed_at: row.try_get(6)?,
                    error: row.try_get(7)?,
                    created_at: row.try_get(8)?,
                })
            },
            mapper: |it| Job::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        EnqueueJobParams<T1>,
        JobQuery<'c, 'a, 's, C, Job, 4>,
        C,
    > for EnqueueJobStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a EnqueueJobParams<T1>,
    ) -> JobQuery<'c, 'a, 's, C, Job, 4> {
        self.bind(
            client,
            &params.id,
            &params.pipeline_run_id,
            &params.stage_name,
            &params.priority,
        )
    }
}
pub struct ClaimJobStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn claim_job() -> ClaimJobStmt {
    ClaimJobStmt(
        "UPDATE job_queue SET status = 'running', claimed_at = NOW(), claimed_by = $1 WHERE id = ( SELECT id FROM job_queue WHERE status = 'pending' ORDER BY priority DESC, created_at ASC LIMIT 1 FOR UPDATE SKIP LOCKED ) RETURNING id, pipeline_run_id, stage_name, priority, status, claimed_by, claimed_at, error, created_at",
        None,
    )
}
impl ClaimJobStmt {
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
        claimed_by: &'a T1,
    ) -> JobQuery<'c, 'a, 's, C, Job, 1> {
        JobQuery {
            client,
            params: [claimed_by],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row: &tokio_postgres::Row| -> Result<JobBorrowed, tokio_postgres::Error> {
                Ok(JobBorrowed {
                    id: row.try_get(0)?,
                    pipeline_run_id: row.try_get(1)?,
                    stage_name: row.try_get(2)?,
                    priority: row.try_get(3)?,
                    status: row.try_get(4)?,
                    claimed_by: row.try_get(5)?,
                    claimed_at: row.try_get(6)?,
                    error: row.try_get(7)?,
                    created_at: row.try_get(8)?,
                })
            },
            mapper: |it| Job::from(it),
        }
    }
}
pub struct CompleteJobStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn complete_job() -> CompleteJobStmt {
    CompleteJobStmt(
        "UPDATE job_queue SET status = 'completed' WHERE id = $1",
        None,
    )
}
impl CompleteJobStmt {
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
pub struct FailJobStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn fail_job() -> FailJobStmt {
    FailJobStmt(
        "UPDATE job_queue SET status = 'failed', error = $1 WHERE id = $2",
        None,
    )
}
impl FailJobStmt {
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
        error: &'a T1,
        id: &'a uuid::Uuid,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[error, id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        FailJobParams<T1>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for FailJobStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a FailJobParams<T1>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.error, &params.id))
    }
}
pub struct RetryJobStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn retry_job() -> RetryJobStmt {
    RetryJobStmt(
        "UPDATE job_queue SET status = 'pending', claimed_by = NULL, claimed_at = NULL, error = $1 WHERE id = $2",
        None,
    )
}
impl RetryJobStmt {
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
        error: &'a T1,
        id: &'a uuid::Uuid,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[error, id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        RetryJobParams<T1>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for RetryJobStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a RetryJobParams<T1>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.error, &params.id))
    }
}
pub struct GetJobByIdStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_job_by_id() -> GetJobByIdStmt {
    GetJobByIdStmt(
        "SELECT id, pipeline_run_id, stage_name, priority, status, claimed_by, claimed_at, error, created_at FROM job_queue WHERE id = $1",
        None,
    )
}
impl GetJobByIdStmt {
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
    ) -> JobQuery<'c, 'a, 's, C, Job, 1> {
        JobQuery {
            client,
            params: [id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row: &tokio_postgres::Row| -> Result<JobBorrowed, tokio_postgres::Error> {
                Ok(JobBorrowed {
                    id: row.try_get(0)?,
                    pipeline_run_id: row.try_get(1)?,
                    stage_name: row.try_get(2)?,
                    priority: row.try_get(3)?,
                    status: row.try_get(4)?,
                    claimed_by: row.try_get(5)?,
                    claimed_at: row.try_get(6)?,
                    error: row.try_get(7)?,
                    created_at: row.try_get(8)?,
                })
            },
            mapper: |it| Job::from(it),
        }
    }
}
pub struct ListJobsByRunStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_jobs_by_run() -> ListJobsByRunStmt {
    ListJobsByRunStmt(
        "SELECT id, pipeline_run_id, stage_name, priority, status, claimed_by, claimed_at, error, created_at FROM job_queue WHERE pipeline_run_id = $1 ORDER BY created_at",
        None,
    )
}
impl ListJobsByRunStmt {
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
        pipeline_run_id: &'a uuid::Uuid,
    ) -> JobQuery<'c, 'a, 's, C, Job, 1> {
        JobQuery {
            client,
            params: [pipeline_run_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row: &tokio_postgres::Row| -> Result<JobBorrowed, tokio_postgres::Error> {
                Ok(JobBorrowed {
                    id: row.try_get(0)?,
                    pipeline_run_id: row.try_get(1)?,
                    stage_name: row.try_get(2)?,
                    priority: row.try_get(3)?,
                    status: row.try_get(4)?,
                    claimed_by: row.try_get(5)?,
                    claimed_at: row.try_get(6)?,
                    error: row.try_get(7)?,
                    created_at: row.try_get(8)?,
                })
            },
            mapper: |it| Job::from(it),
        }
    }
}
pub struct CountPendingJobsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn count_pending_jobs() -> CountPendingJobsStmt {
    CountPendingJobsStmt(
        "SELECT COUNT(*) as count FROM job_queue WHERE status = 'pending'",
        None,
    )
}
impl CountPendingJobsStmt {
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
    ) -> PendingCountQuery<'c, 'a, 's, C, PendingCount, 0> {
        PendingCountQuery {
            client,
            params: [],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row: &tokio_postgres::Row| -> Result<PendingCount, tokio_postgres::Error> {
                Ok(PendingCount {
                    count: row.try_get(0)?,
                })
            },
            mapper: |it| PendingCount::from(it),
        }
    }
}
