use moka::sync::Cache;

pub type MetricsKey = String;
pub type MetricsValue = String;
pub type MetricsCache = Cache<MetricsKey, MetricsValue>;
