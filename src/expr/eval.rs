//! Expression evaluator + environment.

use std::collections::HashMap;

use serde_yaml::Value as Yaml;

use super::ast::{CompareOp, Expr};
use super::funcs;
use crate::model::{GithubContext, JobContext, JobStatus, Needs, StepResult};

/// Runtime value produced by the evaluator.
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0 && !f.is_nan(),
            Value::Str(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(m) => !m.is_empty(),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Str(s) => s.clone(),
            Value::Array(a) => a
                .iter()
                .map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(","),
            Value::Object(_) => serde_json::to_string(self).unwrap_or_default(),
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            Value::Bool(true) => Some(1.0),
            Value::Bool(false) => Some(0.0),
            Value::Str(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(*b),
            Value::Int(i) => serde_json::Value::Number((*i).into()),
            Value::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::Str(s) => serde_json::Value::String(s.clone()),
            Value::Array(a) => {
                serde_json::Value::Array(a.iter().map(|v| v.to_json()).collect())
            }
            Value::Object(m) => serde_json::Value::Object(
                m.iter()
                    .map(|(k, v)| (k.clone(), v.to_json()))
                    .collect(),
            ),
        }
    }

    pub fn from_yaml(v: &serde_yaml::Value) -> Self {
        match v {
            Yaml::Null => Value::Null,
            Yaml::Bool(b) => Value::Bool(*b),
            Yaml::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            Yaml::String(s) => Value::Str(s.clone()),
            Yaml::Sequence(seq) => {
                Value::Array(seq.iter().map(Value::from_yaml).collect())
            }
            Yaml::Mapping(m) => {
                let mut out = HashMap::new();
                for (k, v) in m.iter() {
                    let key = match k {
                        Yaml::String(s) => s.clone(),
                        Yaml::Bool(b) => b.to_string(),
                        Yaml::Number(n) => n.to_string(),
                        _ => "?".to_string(),
                    };
                    out.insert(key, Value::from_yaml(v));
                }
                Value::Object(out)
            }
            Yaml::Tagged(t) => Value::from_yaml(t.value()),
        }
    }
}

/// Evaluation environment — like nektos/act `EvaluationEnvironment`.
#[derive(Default)]
pub struct Env {
    pub github: GithubContext,
    pub env: HashMap<String, String>,
    pub job: JobContext,
    pub steps: HashMap<String, StepResult>,
    pub needs: HashMap<String, Needs>,
    pub secrets: HashMap<String, String>,
    pub vars: HashMap<String, String>,
    pub strategy: HashMap<String, Yaml>,
    pub matrix: HashMap<String, Yaml>,
    pub inputs: HashMap<String, String>,
    pub runner: HashMap<String, String>,
    pub hash_files: Box<dyn Fn(&[String]) -> Vec<String> + Send + Sync>,
}

impl std::fmt::Debug for Env {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Env")
            .field("github", &self.github)
            .field("env", &self.env)
            .field("job", &self.job)
            .field("steps", &self.steps)
            .finish_non_exhaustive()
    }
}

/// Default status check applied to `if:` expressions when none of
/// `success()`, `failure()`, `cancelled()`, `always()` is referenced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultStatusCheck {
    None,
    Success,
    Always,
    Cancelled,
    Failure,
}

impl DefaultStatusCheck {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Success => "success",
            Self::Always => "always",
            Self::Cancelled => "cancelled",
            Self::Failure => "failure",
        }
    }
}

pub struct Evaluator<'a> {
    pub env: &'a Env,
}

impl<'a> Evaluator<'a> {
    pub fn new(env: &'a Env) -> Self {
        Self { env }
    }

    /// Evaluate an Expr, optionally wrapping `success()`/`always()` around
    /// it when needed.
    pub fn evaluate(&self, expr: &Expr, dsc: DefaultStatusCheck) -> anyhow::Result<Value> {
        if dsc != DefaultStatusCheck::None && !self.references_status(expr) {
            let wrapped = Expr::And(
                Box::new(Expr::Call(
                    dsc.as_str().to_string(),
                    Vec::new(),
                )),
                Box::new(expr.clone()),
            );
            return self.eval(&wrapped);
        }
        self.eval(expr)
    }

    fn references_status(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Call(name, _) => {
                matches!(name.as_str(), "success" | "failure" | "cancelled" | "always")
            }
            Expr::Not(e) => self.references_status(e),
            Expr::And(a, b) | Expr::Or(a, b) => self.references_status(a) || self.references_status(b),
            Expr::Compare(_, a, b) => self.references_status(a) || self.references_status(b),
            Expr::Index(a, b) => self.references_status(a) || self.references_status(b),
            Expr::Attr(a, _) => self.references_status(a),
            Expr::ArrayDeref(a) => self.references_status(a),
            _ => false,
        }
    }

    fn eval(&self, expr: &Expr) -> anyhow::Result<Value> {
        Ok(match expr {
            Expr::Null => Value::Null,
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Int(i) => Value::Int(*i),
            Expr::Float(f) => Value::Float(*f),
            Expr::Str(s) => Value::Str(s.clone()),
            Expr::Ident(name) => self.resolve_ident(name)?,
            Expr::Index(base, idx) => {
                let base = self.eval(base)?;
                let idx = self.eval(idx)?;
                self.index(base, idx)?
            }
            Expr::Attr(base, name) => {
                let base = self.eval(base)?;
                self.attr(base, name)
            }
            Expr::ArrayDeref(base) => {
                let base = self.eval(base)?;
                match base {
                    Value::Array(a) => Value::Array(a),
                    v => Value::Array(vec![v]),
                }
            }
            Expr::Not(inner) => {
                let v = self.eval(inner)?;
                Value::Bool(!v.is_truthy())
            }
            Expr::And(a, b) => {
                let lhs = self.eval(a)?;
                if !lhs.is_truthy() {
                    lhs
                } else {
                    self.eval(b)?
                }
            }
            Expr::Or(a, b) => {
                let lhs = self.eval(a)?;
                if lhs.is_truthy() {
                    lhs
                } else {
                    self.eval(b)?
                }
            }
            Expr::Compare(op, a, b) => {
                let lhs = self.eval(a)?;
                let rhs = self.eval(b)?;
                Value::Bool(self.compare(*op, lhs, rhs)?)
            }
            Expr::Call(name, args) => funcs::call(self, name, args)?,
        })
    }

    fn resolve_ident(&self, name: &str) -> anyhow::Result<Value> {
        // Reserved context names first.
        match name {
            "github" => return Ok(github_to_value(&self.env.github)),
            "env" => {
                return Ok(Value::Object(
                    self.env
                        .env
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
                        .collect(),
                ));
            }
            "job" => {
                return Ok(Value::Object(HashMap::from([
                    (
                        "status".into(),
                        Value::Str(self.env.job.status.to_string()),
                    ),
                ])));
            }
            "steps" => {
                return Ok(Value::Object(
                    self.env
                        .steps
                        .iter()
                        .map(|(k, v)| (k.clone(), step_result_to_value(v)))
                        .collect(),
                ));
            }
            "needs" => {
                return Ok(Value::Object(
                    self.env
                        .needs
                        .iter()
                        .map(|(k, v)| (k.clone(), needs_to_value(v)))
                        .collect(),
                ));
            }
            "secrets" => {
                return Ok(Value::Object(
                    self.env
                        .secrets
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
                        .collect(),
                ));
            }
            "vars" => {
                return Ok(Value::Object(
                    self.env
                        .vars
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
                        .collect(),
                ));
            }
            "strategy" => {
                return Ok(Value::Object(
                    self.env
                        .strategy
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::from_yaml(v)))
                        .collect(),
                ));
            }
            "matrix" => {
                return Ok(Value::Object(
                    self.env
                        .matrix
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::from_yaml(v)))
                        .collect(),
                ));
            }
            "inputs" => {
                return Ok(Value::Object(
                    self.env
                        .inputs
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
                        .collect(),
                ));
            }
            "runner" => {
                return Ok(Value::Object(
                    self.env
                        .runner
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
                        .collect(),
                ));
            }
            "infinity" => return Ok(Value::Float(f64::INFINITY)),
            "nan" => return Ok(Value::Float(f64::NAN)),
            _ => {}
        }
        // Otherwise: bare `env.VAR` style or unknown -> empty.
        Ok(if let Some(v) = self.env.env.get(name) {
            Value::Str(v.clone())
        } else {
            Value::Null
        })
    }

    fn attr(&self, base: Value, name: &str) -> Value {
        match base {
            Value::Object(m) => m.get(name).cloned().unwrap_or(Value::Null),
            Value::Array(a) => Value::Array(
                a.iter()
                    .map(|v| self.attr(v.clone(), name))
                    .collect(),
            ),
            Value::Str(s) => Value::Str(s),
            other => Value::Null,
        }
    }

    fn index(&self, base: Value, idx: Value) -> anyhow::Result<Value> {
        match (base, idx) {
            (Value::Array(a), Value::Int(i)) => Ok(a
                .get(i as usize)
                .cloned()
                .unwrap_or(Value::Null)),
            (Value::Object(m), Value::Str(k)) => Ok(m.get(&k).cloned().unwrap_or(Value::Null)),
            (Value::Array(a), Value::Str(k)) => Ok(a
                .iter()
                .find(|v| matches!(v, Value::Str(s) if *s == k))
                .cloned()
                .unwrap_or(Value::Null)),
            _ => Ok(Value::Null),
        }
    }

    fn compare(&self, op: CompareOp, a: Value, b: Value) -> anyhow::Result<bool> {
        let ok = match op {
            CompareOp::Eq => try_eq(&a, &b),
            CompareOp::Ne => !try_eq(&a, &b),
            CompareOp::Lt | CompareOp::Le | CompareOp::Gt | CompareOp::Ge => {
                let (la, lb) = (a.as_number(), b.as_number());
                use CompareOp::*;
                match (la, lb) {
                    (Some(x), Some(y)) => match op {
                        Lt => x < y,
                        Le => x <= y,
                        Gt => x > y,
                        Ge => x >= y,
                        _ => unreachable!(),
                    },
                    _ => {
                        let (x, y) = (a.as_str(), b.as_str());
                        match op {
                            Lt => x < y,
                            Le => x <= y,
                            Gt => x > y,
                            Ge => x >= y,
                            _ => unreachable!(),
                        }
                    }
                }
            }
            _ => unreachable!(),
        };
        Ok(ok)
    }
}

fn try_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Float(y)) | (Value::Float(y), Value::Int(x)) => (*x as f64) == *y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x.eq_ignore_ascii_case(y),
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

fn github_to_value(g: &GithubContext) -> Value {
    // We re-use serde_json as the canonical serializer for `github.*`.
    let json = serde_json::to_value(g).unwrap_or(serde_json::Value::Null);
    serde_json_to_value(json)
}

fn serde_json_to_value(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::Str(s),
        serde_json::Value::Array(a) => {
            Value::Array(a.into_iter().map(serde_json_to_value).collect())
        }
        serde_json::Value::Object(m) => Value::Object(
            m.into_iter()
                .map(|(k, v)| (k, serde_json_to_value(v)))
                .collect(),
        ),
    }
}

fn step_result_to_value(s: &StepResult) -> Value {
    Value::Object(HashMap::from([
        (
            "outputs".into(),
            Value::Object(
                s.outputs
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
                    .collect(),
            ),
        ),
        ("conclusion".into(), Value::Str(s.conclusion.to_string())),
        ("outcome".into(), Value::Str(s.outcome.to_string())),
    ]))
}

fn needs_to_value(n: &Needs) -> Value {
    Value::Object(HashMap::from([
        (
            "outputs".into(),
            Value::Object(
                n.outputs
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::Str(v.clone())))
                    .collect(),
            ),
        ),
        ("result".into(), Value::Str(n.result.to_string())),
    ]))
}

impl From<JobStatus> for Value {
    fn from(s: JobStatus) -> Value {
        Value::Str(s.to_string())
    }
}

impl From<StepStatus> for Value {
    fn from(s: StepStatus) -> Value {
        Value::Str(s.to_string())
    }
}

// Re-exports so funcs.rs can see them.
pub use crate::model::{StepStatus as StepStatusT, JobStatus as JobStatusT};

/// Convenience for evaluating a string `if:` expression.
pub fn eval_if(expr: &str, env: &Env, dsc: DefaultStatusCheck) -> anyhow::Result<bool> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Ok(true);
    }
    let inner = super::interpolate::strip_expr(trimmed);
    let ast = super::parser::parse(inner.trim())?;
    let evaluator = Evaluator::new(env);
    let value = evaluator.evaluate(&ast, dsc)?;
    Ok(value.is_truthy())
}
