//! Istio Service Mesh Operator for Helix Rust SDK
use crate::error::HlxError;
use crate::operators::utils;
use crate::value::Value;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IstioConfig {
    pub namespace: String,
    pub kubeconfig_path: Option<String>,
    pub api_server_url: Option<String>,
    pub token: Option<String>,
    pub ca_cert_path: Option<String>,
    pub timeout: u64,
    pub verify_ssl: bool,
}

impl Default for IstioConfig {
    fn default() -> Self {
        Self {
            namespace: "istio-system".to_string(),
            kubeconfig_path: None,
            api_server_url: None,
            token: None,
            ca_cert_path: None,
            timeout: 30,
            verify_ssl: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualService {
    pub metadata: ResourceMetadata,
    pub spec: VirtualServiceSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualServiceSpec {
    pub hosts: Vec<String>,
    pub gateways: Vec<String>,
    pub http: Vec<HttpRoute>,
    pub tcp: Vec<TcpRoute>,
    pub tls: Vec<TlsRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRoute {
    pub name: Option<String>,
    pub match_conditions: Vec<HttpMatchRequest>,
    pub route: Vec<HttpRouteDestination>,
    pub redirect: Option<HttpRedirect>,
    pub fault: Option<HttpFaultInjection>,
    pub timeout: Option<String>,
    pub retries: Option<HttpRetry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpMatchRequest {
    pub uri: Option<StringMatch>,
    pub method: Option<StringMatch>,
    pub headers: HashMap<String, StringMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringMatch {
    pub exact: Option<String>,
    pub prefix: Option<String>,
    pub regex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRouteDestination {
    pub destination: Destination,
    pub weight: Option<u32>,
    pub headers: Option<Headers>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Destination {
    pub host: String,
    pub subset: Option<String>,
    pub port: Option<PortSelector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortSelector {
    pub number: Option<u32>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Headers {
    pub request: Option<HeaderOperations>,
    pub response: Option<HeaderOperations>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderOperations {
    pub add: HashMap<String, String>,
    pub set: HashMap<String, String>,
    pub remove: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRedirect {
    pub uri: Option<String>,
    pub authority: Option<String>,
    pub redirect_code: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpFaultInjection {
    pub delay: Option<Delay>,
    pub abort: Option<Abort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delay {
    pub percentage: Option<f64>,
    pub fixed_delay: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Abort {
    pub percentage: Option<f64>,
    pub http_status: Option<u32>,
    pub grpc_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRetry {
    pub attempts: u32,
    pub per_try_timeout: Option<String>,
    pub retry_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpRoute {
    pub match_conditions: Vec<L4MatchAttributes>,
    pub route: Vec<RouteDestination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsRoute {
    pub match_conditions: Vec<TlsMatchAttributes>,
    pub route: Vec<RouteDestination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L4MatchAttributes {
    pub destination_subnets: Vec<String>,
    pub port: Option<u32>,
    pub source_labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsMatchAttributes {
    pub sni_hosts: Vec<String>,
    pub destination_subnets: Vec<String>,
    pub port: Option<u32>,
    pub source_labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDestination {
    pub destination: Destination,
    pub weight: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationRule {
    pub metadata: ResourceMetadata,
    pub spec: DestinationRuleSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationRuleSpec {
    pub host: String,
    pub traffic_policy: Option<TrafficPolicy>,
    pub subsets: Vec<Subset>,
    pub export_to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficPolicy {
    pub load_balancer: Option<LoadBalancerSettings>,
    pub connection_pool: Option<ConnectionPoolSettings>,
    pub outlier_detection: Option<OutlierDetection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerSettings {
    pub simple: Option<String>, // ROUND_ROBIN, LEAST_CONN, RANDOM, PASSTHROUGH
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolSettings {
    pub tcp: Option<TcpSettings>,
    pub http: Option<HttpSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpSettings {
    pub max_connections: Option<u32>,
    pub connect_timeout: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSettings {
    pub http1_max_pending_requests: Option<u32>,
    pub http2_max_requests: Option<u32>,
    pub max_requests_per_connection: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlierDetection {
    pub consecutive_errors: Option<u32>,
    pub interval: Option<String>,
    pub base_ejection_time: Option<String>,
    pub max_ejection_percent: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subset {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub traffic_policy: Option<TrafficPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gateway {
    pub metadata: ResourceMetadata,
    pub spec: GatewaySpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySpec {
    pub selector: HashMap<String, String>,
    pub servers: Vec<Server>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub port: Port,
    pub hosts: Vec<String>,
    pub tls: Option<TlsOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub number: u32,
    pub name: String,
    pub protocol: String, // HTTP, HTTPS, GRPC, HTTP2, MONGO, TCP, TLS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsOptions {
    pub mode: String, // SIMPLE, MUTUAL, ISTIO_MUTUAL
    pub credential_name: Option<String>,
    pub private_key: Option<String>,
    pub server_certificate: Option<String>,
    pub ca_certificates: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetadata {
    pub name: String,
    pub namespace: String,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
}

#[derive(Debug, Default)]
struct IstioMetrics {
    virtual_services_created: u64,
    destination_rules_created: u64,
    gateways_created: u64,
    resources_updated: u64,
    resources_deleted: u64,
    api_calls: u64,
    api_errors: u64,
}

pub struct IstioOperator {
    config: IstioConfig,
    metrics: Arc<Mutex<IstioMetrics>>,
    virtual_services: Arc<RwLock<HashMap<String, VirtualService>>>, // name -> resource
    destination_rules: Arc<RwLock<HashMap<String, DestinationRule>>>, // name -> resource
    gateways: Arc<RwLock<HashMap<String, Gateway>>>, // name -> resource
}

impl IstioOperator {
    pub async fn new(config: IstioConfig) -> Result<Self, HlxError> {
        info!("Istio operator initialized for namespace: {}", config.namespace);

        Ok(Self {
            config,
            metrics: Arc::new(Mutex::new(IstioMetrics::default())),
            virtual_services: Arc::new(RwLock::new(HashMap::new())),
            destination_rules: Arc::new(RwLock::new(HashMap::new())),
            gateways: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn create_virtual_service(&self, vs: VirtualService) -> Result<String, HlxError> {
        debug!("Creating virtual service: {}", vs.metadata.name);

        {
            let mut virtual_services = self.virtual_services.write().await;
            virtual_services.insert(vs.metadata.name.clone(), vs.clone());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.virtual_services_created += 1;
            metrics.api_calls += 1;
        }

        info!("Virtual service created: {}", vs.metadata.name);
        Ok(vs.metadata.name)
    }

    pub async fn create_destination_rule(&self, dr: DestinationRule) -> Result<String, HlxError> {
        debug!("Creating destination rule: {}", dr.metadata.name);

        {
            let mut destination_rules = self.destination_rules.write().await;
            destination_rules.insert(dr.metadata.name.clone(), dr.clone());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.destination_rules_created += 1;
            metrics.api_calls += 1;
        }

        info!("Destination rule created: {}", dr.metadata.name);
        Ok(dr.metadata.name)
    }

    pub async fn create_gateway(&self, gw: Gateway) -> Result<String, HlxError> {
        debug!("Creating gateway: {}", gw.metadata.name);

        {
            let mut gateways = self.gateways.write().await;
            gateways.insert(gw.metadata.name.clone(), gw.clone());
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.gateways_created += 1;
            metrics.api_calls += 1;
        }

        info!("Gateway created: {}", gw.metadata.name);
        Ok(gw.metadata.name)
    }

    pub async fn get_virtual_service(&self, name: &str) -> Result<Option<VirtualService>, HlxError> {
        let virtual_services = self.virtual_services.read().await;
        Ok(virtual_services.get(name).cloned())
    }

    pub async fn delete_virtual_service(&self, name: &str) -> Result<bool, HlxError> {
        debug!("Deleting virtual service: {}", name);

        let existed = {
            let mut virtual_services = self.virtual_services.write().await;
            virtual_services.remove(name).is_some()
        };

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.resources_deleted += 1;
            metrics.api_calls += 1;
        }

        if existed {
            info!("Virtual service deleted: {}", name);
        }

        Ok(existed)
    }

    pub async fn apply_traffic_split(&self, service_name: &str, splits: HashMap<String, u32>) -> Result<(), HlxError> {
        debug!("Applying traffic split for service: {} with splits: {:?}", service_name, splits);

        // Create a simple virtual service for traffic splitting
        let routes: Vec<HttpRouteDestination> = splits.into_iter().map(|(subset, weight)| {
            HttpRouteDestination {
                destination: Destination {
                    host: service_name.to_string(),
                    subset: Some(subset),
                    port: None,
                },
                weight: Some(weight),
                headers: None,
            }
        }).collect();

        let vs = VirtualService {
            metadata: ResourceMetadata {
                name: format!("{}-traffic-split", service_name),
                namespace: self.config.namespace.clone(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            spec: VirtualServiceSpec {
                hosts: vec![service_name.to_string()],
                gateways: vec![],
                http: vec![HttpRoute {
                    name: Some("traffic-split".to_string()),
                    match_conditions: vec![],
                    route: routes,
                    redirect: None,
                    fault: None,
                    timeout: None,
                    retries: None,
                }],
                tcp: vec![],
                tls: vec![],
            },
        };

        self.create_virtual_service(vs).await?;

        info!("Traffic split applied for service: {}", service_name);
        Ok(())
    }

    pub async fn configure_circuit_breaker(&self, service_name: &str, max_connections: u32, max_requests: u32) -> Result<(), HlxError> {
        debug!("Configuring circuit breaker for service: {} max_conn: {} max_req: {}", service_name, max_connections, max_requests);

        let dr = DestinationRule {
            metadata: ResourceMetadata {
                name: format!("{}-circuit-breaker", service_name),
                namespace: self.config.namespace.clone(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            spec: DestinationRuleSpec {
                host: service_name.to_string(),
                traffic_policy: Some(TrafficPolicy {
                    load_balancer: Some(LoadBalancerSettings {
                        simple: Some("ROUND_ROBIN".to_string()),
                    }),
                    connection_pool: Some(ConnectionPoolSettings {
                        tcp: Some(TcpSettings {
                            max_connections: Some(max_connections),
                            connect_timeout: Some("30s".to_string()),
                        }),
                        http: Some(HttpSettings {
                            http1_max_pending_requests: Some(max_requests),
                            http2_max_requests: Some(max_requests),
                            max_requests_per_connection: Some(10),
                        }),
                    }),
                    outlier_detection: Some(OutlierDetection {
                        consecutive_errors: Some(5),
                        interval: Some("30s".to_string()),
                        base_ejection_time: Some("30s".to_string()),
                        max_ejection_percent: Some(50),
                    }),
                }),
                subsets: vec![],
                export_to: vec![],
            },
        };

        self.create_destination_rule(dr).await?;

        info!("Circuit breaker configured for service: {}", service_name);
        Ok(())
    }

    pub async fn inject_fault(&self, service_name: &str, delay_percent: f64, delay_duration: &str, abort_percent: f64, abort_status: u32) -> Result<(), HlxError> {
        debug!("Injecting fault for service: {} delay: {}% {}ms abort: {}% {}", 
               service_name, delay_percent, delay_duration, abort_percent, abort_status);

        let vs = VirtualService {
            metadata: ResourceMetadata {
                name: format!("{}-fault-injection", service_name),
                namespace: self.config.namespace.clone(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            spec: VirtualServiceSpec {
                hosts: vec![service_name.to_string()],
                gateways: vec![],
                http: vec![HttpRoute {
                    name: Some("fault-injection".to_string()),
                    match_conditions: vec![],
                    route: vec![HttpRouteDestination {
                        destination: Destination {
                            host: service_name.to_string(),
                            subset: None,
                            port: None,
                        },
                        weight: Some(100),
                        headers: None,
                    }],
                    redirect: None,
                    fault: Some(HttpFaultInjection {
                        delay: Some(Delay {
                            percentage: Some(delay_percent),
                            fixed_delay: Some(delay_duration.to_string()),
                        }),
                        abort: Some(Abort {
                            percentage: Some(abort_percent),
                            http_status: Some(abort_status),
                            grpc_status: None,
                        }),
                    }),
                    timeout: None,
                    retries: None,
                }],
                tcp: vec![],
                tls: vec![],
            },
        };

        self.create_virtual_service(vs).await?;

        info!("Fault injection configured for service: {}", service_name);
        Ok(())
    }

    pub fn get_metrics(&self) -> HashMap<String, Value> {
        let metrics = self.metrics.lock().unwrap();
        let mut result = HashMap::new();

        result.insert("virtual_services_created".to_string(), Value::Number(metrics.virtual_services_created as f64));
        result.insert("destination_rules_created".to_string(), Value::Number(metrics.destination_rules_created as f64));
        result.insert("gateways_created".to_string(), Value::Number(metrics.gateways_created as f64));
        result.insert("resources_updated".to_string(), Value::Number(metrics.resources_updated as f64));
        result.insert("resources_deleted".to_string(), Value::Number(metrics.resources_deleted as f64));
        result.insert("api_calls".to_string(), Value::Number(metrics.api_calls as f64));
        result.insert("api_errors".to_string(), Value::Number(metrics.api_errors as f64));

        if metrics.api_calls > 0 {
            let success_rate = ((metrics.api_calls - metrics.api_errors) as f64 / metrics.api_calls as f64) * 100.0;
            result.insert("api_success_rate_percent".to_string(), Value::Number(success_rate));
        }

        result
    }
}

#[async_trait]
impl crate::operators::OperatorTrait for IstioOperator {
    async fn execute(&self, operator: &str, params: &str) -> Result<Value, HlxError> {
        let params_map = utils::parse_params(params)?;

        match operator {
            "traffic_split" => {
                let service_name = params_map.get("service_name").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("service_name".to_string()),
                        message: "Missing service name".to_string(),
                    })?;

                let splits = params_map.get("splits").and_then(|v| {
                    if let Value::Object(obj) = v {
                        let mut splits_map = HashMap::new();
                        for (k, v) in obj {
                            if let Some(weight) = v.as_number() {
                                splits_map.insert(k.clone(), weight as u32);
                            }
                        }
                        Some(splits_map)
                    } else { None }
                }).unwrap_or_default();

                self.apply_traffic_split(&service_name, splits).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("service_name".to_string(), Value::String(service_name.to_string(.to_string())));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "circuit_breaker" => {
                let service_name = params_map.get("service_name").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("service_name".to_string()),
                        message: "Missing service name".to_string(),
                    })?;

                let max_connections = params_map.get("max_connections").and_then(|v| v.as_number())
                    .unwrap_or(100.0) as u32;

                let max_requests = params_map.get("max_requests").and_then(|v| v.as_number())
                    .unwrap_or(10.0) as u32;

                self.configure_circuit_breaker(&service_name, max_connections, max_requests).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("service_name".to_string(), Value::String(service_name.to_string(.to_string())));
                    map.insert("max_connections".to_string(), Value::Number(max_connections as f64));
                    map.insert("max_requests".to_string(), Value::Number(max_requests as f64));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "fault_injection" => {
                let service_name = params_map.get("service_name").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("service_name".to_string()),
                        message: "Missing service name".to_string(),
                    })?;

                let delay_percent = params_map.get("delay_percent").and_then(|v| v.as_number()).unwrap_or(0.0);
                let delay_duration = params_map.get("delay_duration").and_then(|v| v.as_string())
                    .unwrap_or_else(|| "100ms");
                let abort_percent = params_map.get("abort_percent").and_then(|v| v.as_number()).unwrap_or(0.0);
                let abort_status = params_map.get("abort_status").and_then(|v| v.as_number()).unwrap_or(500.0) as u32;

                self.inject_fault(&service_name, delay_percent, &delay_duration, abort_percent, abort_status).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("service_name".to_string(), Value::String(service_name.to_string(.to_string())));
                    map.insert("delay_percent".to_string(), Value::Number(delay_percent));
                    map.insert("abort_percent".to_string(), Value::Number(abort_percent));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "delete_virtual_service" => {
                let name = params_map.get("name").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("name".to_string()),
                        message: "Missing virtual service name".to_string(),
                    })?;

                let deleted = self.delete_virtual_service(&name).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("name".to_string(), Value::String(name.to_string(.to_string())));
                    map.insert("deleted".to_string(), Value::Boolean(deleted));
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "get_virtual_service" => {
                let name = params_map.get("name").and_then(|v| v.as_string())
                    .ok_or_else(|| HlxError::ValidationError { message: "Validation failed".to_string(), field: None, value: None, rule: None,
                        value: Some("".to_string()),
                        rule: Some("required".to_string()),
                        field: Some("name".to_string()),
                        message: "Missing virtual service name".to_string(),
                    })?;

                let vs = self.get_virtual_service(&name).await?;

                Ok(Value::Object({
                    let mut map = HashMap::new();
                    map.insert("name".to_string(), Value::String(name.to_string(.to_string())));
                    if let Some(virtual_service) = vs {
                        map.insert("namespace".to_string(), Value::String(virtual_service.metadata.namespace.to_string()));
                        map.insert("hosts".to_string(), Value::Array(
                            virtual_service.spec.hosts.into_iter().map(Value::String).collect()
                        ));
                        map.insert("http_routes".to_string(), Value::Number(virtual_service.spec.http.len() as f64));
                    } else {
                        map.insert("virtual_service".to_string(), Value::Null);
                    }
                    map.insert("success".to_string(), Value::Boolean(true));
                    map
                }))
            }

            "metrics" => {
                let metrics = self.get_metrics();
                Ok(Value::Object(metrics))
            }

            _ => Err(HlxError::InvalidParameters {
                operator: "istio".to_string(),
                params: format!("Unknown Istio operation: {}", operator),
            }),
        }
    }
} 