use std::sync::Mutex;

use prometheus_client::{
    collector::Collector,
    encoding::EncodeMetric,
    metrics::{counter::Counter, gauge::Gauge},
    registry::{Registry, Unit},
};
use tokio_metrics::{RuntimeIntervals, RuntimeMonitor};

/// Register the Tokio Metrics collector with a Prometheus [`Registry`].
///
/// ## Example
///
/// ```
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # rt.block_on(async {
/// let handle = tokio::runtime::Handle::current();
/// let runtime_monitor = tokio_metrics::RuntimeMonitor::new(&handle);
/// let mut registry = prometheus_client::registry::Registry::default();
/// tokio_prometheus_client::register(runtime_monitor, registry.sub_registry_with_prefix("tokio"));
/// # });
/// ```
pub fn register(monitor: RuntimeMonitor, registry: &mut Registry) {
    registry.register_collector(Box::new(RuntimeCollector::new(monitor)))
}

/// Collects tokio runtime metrics
#[derive(Debug)]
struct RuntimeCollector {
    metrics: RuntimeMetrics,
    intervals: Mutex<RuntimeIntervals>,
}

impl RuntimeCollector {
    /// Create a [`RuntimeCollector`] in namespace.
    pub fn new(monitor: RuntimeMonitor) -> Self {
        let intervals = Mutex::new(monitor.intervals());
        let metrics = RuntimeMetrics::default();
        Self { metrics, intervals }
    }
}
impl Collector for RuntimeCollector {
    fn encode(
        &self,
        mut encoder: prometheus_client::encoding::DescriptorEncoder,
    ) -> Result<(), std::fmt::Error> {
        // Helper macros to ensure the metric name is consistent
        macro_rules! encode {
            ($name:ident, $description:expr, $unit:expr, $encoder:expr,) => {
                let metric_encoder = $encoder.encode_descriptor(
                    stringify!($name),
                    $description,
                    $unit,
                    self.metrics.$name.metric_type(),
                )?;
                self.metrics.$name.encode(metric_encoder)?;
            };
        }

        let interval = self
            .intervals
            .lock()
            .expect("should be able to lock intervals")
            .next()
            .expect("should always be another interval");

        self.metrics.update(interval);

        encode!(
            workers_count,
            "The number of worker threads used by the runtime",
            None,
            encoder,
        );
        encode!(
            total_park_count,
            "The number of times worker threads parked",
            None,
            encoder,
        );
        encode!(
            total_noop_count,
            "The number of times worker threads unparked but performed no work before parking again",
            None,
            encoder,
        );
        encode!(
            total_steal_count,
            "The number of tasks worker threads stole from another worker thread",
            None,
            encoder,
        );
        encode!(
            total_steal_operations,
            "The number of times worker threads stole tasks from another worker thread",
            None,
            encoder,
        );
        encode!(
            num_remote_schedules,
            "The number of tasks scheduled from **outside** of the runtime",
            None,
            encoder,
        );
        encode!(
            total_local_schedule_count,
            "The number of tasks scheduled from worker threads",
            None,
            encoder,
        );
        encode!(
            total_overflow_count,
            "The number of times worker threads saturated their local queues",
            None,
            encoder,
        );
        encode!(
            total_polls_count,
            "The number of tasks that have been polled across all worker threads",
            None,
            encoder,
        );
        encode!(
            total_busy_duration,
            "The amount of time worker threads were busy",
            Some(&Unit::Seconds),
            encoder,
        );
        encode!(
            injection_queue_depth,
            "The number of tasks currently scheduled in the runtime's injection queue",
            None,
            encoder,
        );
        encode!(
            total_local_queue_depth,
            "The total number of tasks currently scheduled in workers' local queues",
            None,
            encoder,
        );
        encode!(
            budget_forced_yield_count,
            "Returns the number of times that tasks have been forced to yield back to the scheduler after exhausting their task budgets",
            None,
            encoder,
        );
        encode!(
            io_driver_ready_count,
            "Returns the number of ready events processed by the runtime’s I/O driver",
            None,
            encoder,
        );

        Ok(())
    }
}

// Current RuntimeMetrics
// https://docs.rs/tokio-metrics/latest/tokio_metrics/struct.RuntimeMetrics.html
#[derive(Debug, Default)]
struct RuntimeMetrics {
    workers_count: Gauge,
    total_park_count: Counter,
    total_noop_count: Counter,
    total_steal_count: Counter,
    total_steal_operations: Counter,
    num_remote_schedules: Counter,
    total_local_schedule_count: Counter,
    total_overflow_count: Counter,
    total_polls_count: Counter,
    total_busy_duration: Counter<f64>,
    injection_queue_depth: Gauge,
    total_local_queue_depth: Gauge,
    budget_forced_yield_count: Counter,
    io_driver_ready_count: Counter,
}

impl RuntimeMetrics {
    fn update(&self, data: tokio_metrics::RuntimeMetrics) {
        // macros to ensure we are using consistent metrics names
        macro_rules! inc_by {
            ( $field:ident, "int" ) => {{
                self.$field.inc_by(data.$field as u64);
            }};
            ( $field:ident, "duration" ) => {{
                self.$field.inc_by(data.$field.as_secs_f64());
            }};
        }
        macro_rules! set {
            ( $field:ident) => {{
                self.$field.set(data.$field as i64);
            }};
        }

        set!(workers_count);
        inc_by!(total_park_count, "int");
        inc_by!(total_noop_count, "int");
        inc_by!(total_steal_count, "int");
        inc_by!(total_steal_operations, "int");
        inc_by!(num_remote_schedules, "int");
        inc_by!(total_local_schedule_count, "int");
        inc_by!(total_overflow_count, "int");
        inc_by!(total_polls_count, "int");
        inc_by!(total_busy_duration, "duration");
        set!(injection_queue_depth);
        set!(total_local_queue_depth);
        inc_by!(budget_forced_yield_count, "int");
        inc_by!(io_driver_ready_count, "int");
    }
}
