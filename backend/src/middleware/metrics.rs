use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error, web,
};
use actix_web::middleware::Next;
use std::time::Instant;
use prometheus::{Counter, Histogram, Registry, Opts, Gauge};
use std::sync::Arc;

/// Prometheus metrics for API endpoints
pub struct ApiMetrics {
    pub request_counter: Counter,
    pub request_duration: Histogram,
    pub error_counter: Counter,
    pub active_connections: Gauge,
}

impl ApiMetrics {
    pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
        let request_counter = Counter::with_opts(Opts::new(
            "api_requests_total",
            "Total number of API requests"
        ))?;
        
        let request_duration = Histogram::with_opts(prometheus::HistogramOpts::new(
            "api_request_duration_seconds",
            "API request duration in seconds"
        ))?;
        
        let error_counter = Counter::with_opts(Opts::new(
            "api_errors_total",
            "Total number of API errors"
        ))?;
        
        let active_connections = Gauge::with_opts(Opts::new(
            "api_active_connections",
            "Number of active API connections"
        ))?;

        registry.register(Box::new(request_counter.clone()))?;
        registry.register(Box::new(request_duration.clone()))?;
        registry.register(Box::new(error_counter.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;

        Ok(Self {
            request_counter,
            request_duration,
            error_counter,
            active_connections,
        })
    }
}

/// Metrics middleware function
pub async fn metrics_middleware(
    req: ServiceRequest,
    next: Next<ServiceRequest>,
) -> Result<ServiceResponse, Error> {
    let start_time = Instant::now();
    let method = req.method().clone();
    let path = req.path().to_string();
    
    // Get metrics from app data
    if let Some(metrics) = req.app_data::<web::Data<Arc<ApiMetrics>>>() {
        // Increment request counter
        metrics.request_counter.inc();
        
        // Increment active connections
        metrics.active_connections.inc();
        
        // Process request
        let result = next.call(req).await;
        let duration = start_time.elapsed();
        
        // Record duration
        metrics.request_duration.observe(duration.as_secs_f64());
        
        // Decrement active connections
        metrics.active_connections.dec();
        
        // Increment error counter if request failed
        if result.is_err() {
            metrics.error_counter.inc();
        }
        
        result
    } else {
        next.call(req).await
    }
}
