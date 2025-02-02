/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex, RwLock};

use http::Uri;
use tokio::sync::mpsc;
use tonic::codegen::InterceptedService;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::service::Interceptor;
use tonic::transport::Channel;
use tonic::Status;

use crate::proto::beam_api::fn_execution::instruction_request;
use crate::proto::beam_api::fn_execution::{
    beam_fn_control_client::BeamFnControlClient, FinalizeBundleRequest,
    GetProcessBundleDescriptorRequest, HarnessMonitoringInfosRequest, InstructionResponse,
    MonitoringInfosMetadataRequest, ProcessBundleDescriptor, ProcessBundleProgressRequest,
    ProcessBundleRequest, ProcessBundleSplitRequest, RegisterRequest,
};
use crate::proto::beam_api::pipeline::PTransform;

use crate::worker::operators::{create_operator, Operator, OperatorContext, OperatorI, Receiver};

#[derive(Clone)]
struct WorkerIdInterceptor {
    id: MetadataValue<Ascii>,
}

impl WorkerIdInterceptor {
    fn new(id: String) -> Self {
        Self {
            id: id.parse().unwrap(),
        }
    }
}

impl Interceptor for WorkerIdInterceptor {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        request.metadata_mut().insert("worker_id", self.id.clone());
        Ok(request)
    }
}

type BundleDescriptorId = String;
#[allow(unused)] // FIXME
type InstructionId = String;

// TODO(sjvanrossum): Convert simple map caches to concurrent caches.
// Using concurrent caches removes the need to synchronize on the worker instance in every context.
#[derive(Debug)]
#[allow(unused)] // FIXME
pub struct Worker {
    // Cheap and safe to clone
    control_client: BeamFnControlClient<InterceptedService<Channel, WorkerIdInterceptor>>,
    // Cheap and safe to clone
    process_bundle_descriptors:
        moka::future::Cache<BundleDescriptorId, Arc<ProcessBundleDescriptor>>,
    bundle_processors: HashMap<String, BundleProcessor>,
    active_bundle_processors: HashMap<String, BundleProcessor>,
    id: String,
    endpoints: WorkerEndpoints,
    options: HashMap<String, String>,
}

impl Worker {
    // TODO(sjvanrossum): Remove Arc and Mutex once the worker's state uses
    // concurrent data structures and/or finer grained locks.
    pub async fn new(id: String, endpoints: WorkerEndpoints) -> Arc<Mutex<Worker>> {
        // TODO: parse URIs in the endpoint struct
        let channel = Channel::builder(endpoints.get_endpoint().parse::<Uri>().unwrap())
            .connect()
            .await
            .expect("Failed to connect to control service");
        let client =
            BeamFnControlClient::with_interceptor(channel, WorkerIdInterceptor::new(id.clone()));

        Arc::new(Mutex::new(Self {
            control_client: client,
            // TODO(sjvanrossum): Maybe define the eviction policy
            process_bundle_descriptors: moka::future::Cache::builder().build(),
            bundle_processors: HashMap::new(),
            active_bundle_processors: HashMap::new(),
            id,
            endpoints,
            options: HashMap::new(),
        }))
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let (control_res_tx, mut control_res_rx) = mpsc::channel::<InstructionResponse>(100);

        let outbound = async_stream::stream! {
            while let Some(control_res) = control_res_rx.recv().await {
                yield control_res
            }
        };
        let response = self.control_client.control(outbound).await?;
        let mut inbound = response.into_inner();

        while let Some(control_req) = inbound.message().await? {
            match control_req.request {
                Some(instruction_request::Request::ProcessBundle(instr_req)) => {
                    self.process_bundle(instr_req);
                }
                Some(instruction_request::Request::ProcessBundleProgress(instr_req)) => {
                    self.process_bundle_progress(instr_req);
                }
                Some(instruction_request::Request::ProcessBundleSplit(instr_req)) => {
                    self.process_bundle_split(instr_req);
                }
                Some(instruction_request::Request::FinalizeBundle(instr_req)) => {
                    self.finalize_bundle(instr_req);
                }
                Some(instruction_request::Request::MonitoringInfos(instr_req)) => {
                    self.monitoring_infos(instr_req);
                }
                Some(instruction_request::Request::HarnessMonitoringInfos(instr_req)) => {
                    self.harness_monitoring_infos(instr_req);
                }
                Some(instruction_request::Request::Register(instr_req)) => {
                    self.register(instr_req);
                }
                _ => {
                    control_res_tx
                        .send(InstructionResponse {
                            instruction_id: control_req.instruction_id.clone(),
                            error: format!("Unexpected request: {:?}", control_req),
                            response: None,
                        })
                        .await?;
                }
            };
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        todo!()
    }

    fn process_bundle(&self, request: ProcessBundleRequest) {
        let mut client = self.control_client.clone();
        let descriptor_cache = self.process_bundle_descriptors.clone();
        tokio::spawn(async move {
            #[allow(unused)] // FIXME
            let descriptor = descriptor_cache
                .try_get_with::<_, Status>(request.process_bundle_descriptor_id.clone(), async {
                    let res = client
                        .get_process_bundle_descriptor(GetProcessBundleDescriptorRequest {
                            process_bundle_descriptor_id: request
                                .process_bundle_descriptor_id
                                .clone(),
                        })
                        .await?;
                    Ok(Arc::new(res.into_inner()))
                })
                .await
                .unwrap();
            // TODO(sjvanrossum): Fetch bundle processor and process bundle
        });
    }

    #[allow(unused)] // FIXME
    fn process_bundle_progress(&self, request: ProcessBundleProgressRequest) {
        // TODO(sjvanrossum): Flesh out after process_bundle is sufficiently implemented
    }

    #[allow(unused)] // FIXME
    fn process_bundle_split(&self, request: ProcessBundleSplitRequest) {
        // TODO(sjvanrossum): Flesh out after process_bundle is sufficiently implemented
    }

    #[allow(unused)] // FIXME
    fn finalize_bundle(&self, request: FinalizeBundleRequest) {
        // TODO(sjvanrossum): Flesh out after process_bundle is sufficiently implemented.
    }

    #[allow(unused)] // FIXME
    fn monitoring_infos(&self, request: MonitoringInfosMetadataRequest) {
        // TODO: Implement
    }

    #[allow(unused)] // FIXME
    fn harness_monitoring_infos(&self, request: HarnessMonitoringInfosRequest) {
        // TODO: Implement
    }

    #[allow(unused)] // FIXME
    fn register(&self, request: RegisterRequest) {
        // TODO: Implement or maybe respond with a failure since this is deprecated
    }
}

#[derive(Clone, Debug)]
pub struct WorkerEndpoints {
    control_endpoint_url: Option<String>,
}

impl WorkerEndpoints {
    pub fn new(control_endpoint_url: Option<String>) -> Self {
        Self {
            control_endpoint_url,
        }
    }

    pub fn get_endpoint(&self) -> &str {
        self.control_endpoint_url.as_ref().unwrap()
    }
}

#[derive(Debug)]
pub struct BundleProcessor {
    descriptor: Arc<ProcessBundleDescriptor>,
    creation_ordered_operators: Mutex<Vec<Arc<Operator>>>,
    topologically_ordered_operators: RwLock<Vec<Arc<Operator>>>,

    operators: RwLock<HashMap<String, Arc<Operator>>>,
    receivers: RwLock<HashMap<String, Arc<Receiver>>>,
    consumers: RwLock<HashMap<String, Vec<String>>>,

    current_bundle_id: Mutex<Option<String>>,
}

impl BundleProcessor {
    pub fn new(descriptor: Arc<ProcessBundleDescriptor>, root_urns: &[&'static str]) -> Arc<Self> {
        let mut consumers: HashMap<String, Vec<String>> = HashMap::new();
        for (transform_id, ptransform) in descriptor.transforms.iter() {
            if is_primitive(ptransform) {
                ptransform
                    .inputs
                    .values()
                    .for_each(|pcollection_id: &String| {
                        if consumers.get(pcollection_id).is_none() {
                            consumers.insert(pcollection_id.clone(), Vec::new());
                        }

                        consumers
                            .get_mut(pcollection_id)
                            .unwrap()
                            .push(transform_id.clone());
                    })
            }
        }

        let mut _instance = Arc::new(Self {
            descriptor: descriptor.clone(),
            creation_ordered_operators: Mutex::new(Vec::new()),
            topologically_ordered_operators: RwLock::new(Vec::new()),
            operators: RwLock::new(HashMap::new()),
            receivers: RwLock::new(HashMap::new()),
            consumers: RwLock::new(consumers),
            current_bundle_id: Mutex::new(None),
        });

        descriptor
            .transforms
            .iter()
            .for_each(|(transform_id, transform)| {
                if let Some(spec) = &transform.spec {
                    if root_urns.contains(&spec.urn.as_str()) {
                        BundleProcessor::get_operator(_instance.clone(), transform_id.clone());
                    }
                }
            });

        _instance.create_topologically_ordered_operators();

        _instance
    }

    // TODO: the bundle processors are being be passed around by value as parameters
    // to avoid closures with temp references in an async context. However, this
    // should be revisited later.
    fn get_receiver(
        bundle_processor: Arc<BundleProcessor>,
        pcollection_id: String,
    ) -> Arc<Receiver> {
        let receivers = bundle_processor.receivers.read().unwrap();

        let receiver = match receivers.get(&pcollection_id) {
            Some(rec) => rec.clone(),
            None => {
                let consumers = bundle_processor.consumers.read().unwrap();

                let pcoll_operators: Vec<Arc<Operator>> = match consumers.get(&pcollection_id) {
                    Some(v) => {
                        drop(receivers);

                        let mut oprs = Vec::new();
                        for transform_id in v {
                            let cl = bundle_processor.clone();
                            oprs.push(BundleProcessor::get_operator(cl, transform_id.clone()))
                        }
                        oprs
                    }
                    None => Vec::new(),
                };

                let receiver = Receiver::new(pcoll_operators);

                let mut receivers = bundle_processor.receivers.write().unwrap();
                receivers.insert(pcollection_id.clone(), Arc::new(receiver));
                receivers[&pcollection_id].clone()
            }
        };

        receiver
    }

    fn get_operator(bundle_processor: Arc<BundleProcessor>, transform_id: String) -> Arc<Operator> {
        let get_receiver = |b: Arc<BundleProcessor>, pcollection_id: String| {
            BundleProcessor::get_receiver(b, pcollection_id)
        };

        let operators = bundle_processor.operators.read().unwrap();

        let operator_opt = operators.get(&transform_id);

        if let Some(arc) = operator_opt {
            arc.clone()
        } else {
            let descriptor = bundle_processor.descriptor.clone();
            drop(operators);

            let op = create_operator(
                &transform_id,
                Arc::new(OperatorContext {
                    descriptor,
                    get_receiver: Box::new(get_receiver),

                    bundle_processor: bundle_processor.clone(),
                }),
            );

            let mut operators = bundle_processor.operators.write().unwrap();
            operators.insert(transform_id.clone(), Arc::new(op));

            let mut creation_ordered_operators =
                bundle_processor.creation_ordered_operators.lock().unwrap();
            creation_ordered_operators.push(operators[&transform_id].clone());

            operators[&transform_id].clone()
        }
    }

    fn create_topologically_ordered_operators(&self) {
        let creation_ordered = self.creation_ordered_operators.lock().unwrap();

        let mut instance_operators = self.topologically_ordered_operators.write().unwrap();

        creation_ordered
            .iter()
            .rev()
            .for_each(|op: &Arc<Operator>| {
                instance_operators.push(op.clone());
            });
    }

    pub async fn process(&self, instruction_id: String) {
        let mut current_bundle_id = self.current_bundle_id.lock().unwrap();
        current_bundle_id.replace(instruction_id);
        drop(current_bundle_id);

        let topologically_ordered_operators = self.topologically_ordered_operators.read().unwrap();
        for o in topologically_ordered_operators.iter().rev() {
            o.start_bundle();
        }

        for o in topologically_ordered_operators.iter() {
            o.finish_bundle();
        }
    }
}

// TODO
#[allow(unused)] // FIXME
fn is_primitive(transform: &PTransform) -> bool {
    true
}
