use super::AiError;
use crate::database::Database;
use std::time::Instant;

pub struct Context<'a> {
    pub database: &'a Database,
    pub task: Option<String>,
    pub stage: String,
}
#[derive(Default)]
pub struct Metrics {
    pub first_response_ms: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}
pub struct Call<'a> {
    db: Option<&'a Database>,
    id: Option<String>,
    started: Instant,
    pub metrics: Metrics,
    finished: bool,
}
impl<'a> Call<'a> {
    pub fn start(context: Option<&Context<'a>>, model: &str) -> Result<Self, AiError> {
        let id = context
            .map(|ctx| {
                ctx.database
                    .begin_model_call(ctx.task.as_deref(), model, &ctx.stage)
            })
            .transpose()
            .map_err(AiError::database)?;
        Ok(Self {
            db: context.map(|ctx| ctx.database),
            id,
            started: Instant::now(),
            metrics: Metrics::default(),
            finished: false,
        })
    }
    pub fn response(&mut self) {
        self.metrics
            .first_response_ms
            .get_or_insert(self.started.elapsed().as_millis() as u64);
    }
    pub fn usage(&mut self, value: &serde_json::Value) {
        let token = |a: &str, b: &str| {
            value
                .get(a)
                .or_else(|| value.get(b))
                .and_then(|v| v.as_u64())
                .filter(|n| *n <= i64::MAX as u64)
        };
        if let Some(n) = token("prompt_tokens", "input_tokens") {
            self.metrics.input_tokens = Some(n);
        }
        if let Some(n) = token("completion_tokens", "output_tokens") {
            self.metrics.output_tokens = Some(n);
        }
        if let Some(n) = token("total_tokens", "total_tokens") {
            self.metrics.total_tokens = Some(n);
        }
    }
    pub fn finish<T>(&mut self, result: &Result<T, AiError>) -> Result<(), AiError> {
        let status = if result.is_ok() {
            "succeeded"
        } else {
            "failed"
        };
        if let (Some(db), Some(id)) = (self.db, &self.id) {
            db.finish_model_call(
                id,
                status,
                self.started.elapsed().as_millis() as u64,
                &self.metrics,
                result.as_ref().err().map(|e| e.code.as_str()),
            )
            .map_err(AiError::database)?;
        }
        self.finished = true;
        Ok(())
    }
}
impl Drop for Call<'_> {
    fn drop(&mut self) {
        if !self.finished {
            if let (Some(db), Some(id)) = (self.db, &self.id) {
                let _ = db.finish_model_call(
                    id,
                    "cancelled",
                    self.started.elapsed().as_millis() as u64,
                    &self.metrics,
                    Some("cancelled"),
                );
            }
        }
    }
}
