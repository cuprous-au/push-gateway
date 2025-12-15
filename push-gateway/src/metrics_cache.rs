use std::fmt::Write;

use moka::sync::Cache;
use nom_openmetrics::Label;

#[derive(Hash, PartialEq, Eq)]
pub struct MetricsKey {
    pub name: String,
    pub labels: Vec<(String, String)>,
}

impl MetricsKey {
    pub fn with_nom_name_and_labels<'a>(name: &'a str, labels: &[Label<'a>]) -> Self {
        let mut labels: Vec<_> = labels
            .iter()
            .map(|label| (label.name.to_string(), label.value.clone()))
            .collect();
        labels.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            name: name.to_string(),
            labels,
        }
    }
}

impl std::fmt::Display for MetricsKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)?;
        if !self.labels.is_empty() {
            f.write_char('{')?;
            let mut first = true;
            for (name, value) in &self.labels {
                if !first {
                    f.write_char(',')?;
                }
                f.write_fmt(format_args!("{}={}", name, value))?;
                first = false;
            }
            f.write_char('}')?;
        }
        Ok(())
    }
}

pub type MetricsValue = f64;
pub type MetricsCache = Cache<MetricsKey, MetricsValue>;
