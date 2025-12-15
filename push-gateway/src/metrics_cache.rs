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

pub type MetricsValue = f64;
pub type MetricsCache = Cache<MetricsKey, MetricsValue>;

#[derive(Hash, PartialEq, Eq)]
pub struct FamiliesKey {
    pub job: String,
    pub labels: Vec<(String, String)>,
}

impl FamiliesKey {
    pub fn new(job: String, mut labels: Vec<(String, String)>) -> Self {
        labels.sort_by(|a, b| a.0.cmp(&b.0));
        Self { job, labels }
    }
}

#[derive(Clone)]
pub struct FamiliesValue {
    pub descriptors: String,
    pub metrics_cache: MetricsCache,
}

pub type FamiliesCache = Cache<FamiliesKey, FamiliesValue>;
