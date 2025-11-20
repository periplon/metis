use prometheus::{
    Counter, CounterVec, Encoder, Gauge, HistogramOpts, HistogramVec, Opts, Registry,
    TextEncoder,
};
use std::sync::Arc;

pub struct MetricsCollector {
    registry: Registry,
    
    // Request metrics
    pub requests_total: CounterVec,
    pub request_duration: HistogramVec,
    pub requests_in_flight: Gauge,
    
    // Strategy metrics
    pub strategy_executions: CounterVec,
    pub strategy_errors: CounterVec,
    pub strategy_duration: HistogramVec,
    
    // Cache metrics
    pub cache_hits: Counter,
    pub cache_misses: Counter,
}

impl MetricsCollector {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Registry::new();
        
        // Request metrics
        let requests_total = CounterVec::new(
            Opts::new("metis_requests_total", "Total number of requests"),
            &["method", "endpoint", "status"],
        )?;
        registry.register(Box::new(requests_total.clone()))?;
        
        let request_duration = HistogramVec::new(
            HistogramOpts::new("metis_request_duration_seconds", "Request duration in seconds"),
            &["method", "endpoint"],
        )?;
        registry.register(Box::new(request_duration.clone()))?;
        
        let requests_in_flight = Gauge::new(
            "metis_requests_in_flight",
            "Number of requests currently being processed",
        )?;
        registry.register(Box::new(requests_in_flight.clone()))?;
        
        // Strategy metrics
        let strategy_executions = CounterVec::new(
            Opts::new("metis_strategy_executions_total", "Total strategy executions"),
            &["strategy"],
        )?;
        registry.register(Box::new(strategy_executions.clone()))?;
        
        let strategy_errors = CounterVec::new(
            Opts::new("metis_strategy_errors_total", "Total strategy errors"),
            &["strategy", "error_type"],
        )?;
        registry.register(Box::new(strategy_errors.clone()))?;
        
        let strategy_duration = HistogramVec::new(
            HistogramOpts::new("metis_strategy_duration_seconds", "Strategy execution duration"),
            &["strategy"],
        )?;
        registry.register(Box::new(strategy_duration.clone()))?;
        
        // Cache metrics
        let cache_hits = Counter::new("metis_cache_hits_total", "Total cache hits")?;
        registry.register(Box::new(cache_hits.clone()))?;
        
        let cache_misses = Counter::new("metis_cache_misses_total", "Total cache misses")?;
        registry.register(Box::new(cache_misses.clone()))?;
        
        Ok(Self {
            registry,
            requests_total,
            request_duration,
            requests_in_flight,
            strategy_executions,
            strategy_errors,
            strategy_duration,
            cache_hits,
            cache_misses,
        })
    }
    
    pub fn encode(&self) -> anyhow::Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics collector")
    }
}

pub struct MetricsHandler {
    collector: Arc<MetricsCollector>,
}

impl MetricsHandler {
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        Self { collector }
    }
    
    pub async fn metrics(&self) -> String {
        self.collector.encode().unwrap_or_else(|e| {
            tracing::error!("Failed to encode metrics: {}", e);
            String::from("# Error encoding metrics\n")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert!(collector.is_ok());
    }
    
    #[test]
    fn test_metrics_encoding() {
        let collector = MetricsCollector::new().unwrap();
        
        // Increment some metrics
        collector.requests_total.with_label_values(&["GET", "/health", "200"]).inc();
        collector.cache_hits.inc();
        
        let encoded = collector.encode();
        assert!(encoded.is_ok());
        
        let metrics_text = encoded.unwrap();
        assert!(metrics_text.contains("metis_requests_total"));
        assert!(metrics_text.contains("metis_cache_hits_total"));
    }
    
    #[tokio::test]
    async fn test_metrics_handler() {
        let collector = Arc::new(MetricsCollector::new().unwrap());
        let handler = MetricsHandler::new(collector.clone());
        
        // Increment a metric
        collector.requests_total.with_label_values(&["POST", "/mcp", "200"]).inc();
        
        let metrics = handler.metrics().await;
        assert!(metrics.contains("metis_requests_total"));
    }
}
