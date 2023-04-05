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

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

//use std::borrow::{Borrow, BorrowMut};

use once_cell::sync::Lazy;
use serde_json;

use crate::internals::serialize;
use crate::internals::urns;
use crate::proto::beam_api::fn_execution::ProcessBundleDescriptor;
use crate::proto::beam_api::pipeline::PTransform;

use crate::worker::sdk_worker::BundleProcessor;
use crate::worker::test_utils::RECORDING_OPERATOR_LOGS;

type OperatorMap = HashMap<&'static str, OperatorDiscriminants>;

static OPERATORS_BY_URN: Lazy<Mutex<OperatorMap>> = Lazy::new(|| {
    // TODO: these will have to be parameterized depending on things such as the runner used
    let m: OperatorMap = HashMap::from([
        // Test operators
        (urns::CREATE_URN, OperatorDiscriminants::Create),
        (urns::RECORDING_URN, OperatorDiscriminants::Recording),
        (urns::PARTITION_URN, OperatorDiscriminants::_Partitioning),
        (urns::IMPULSE_URN, OperatorDiscriminants::Impulse),
        (urns::GROUP_BY_KEY_URN, OperatorDiscriminants::GroupByKey),
        // Production operators
        (urns::DATA_INPUT_URN, OperatorDiscriminants::_DataSource),
        (urns::PAR_DO_URN, OperatorDiscriminants::ParDo),
        (urns::FLATTEN_URN, OperatorDiscriminants::Flatten),
    ]);

    Mutex::new(m)
});

pub trait OperatorI {
    fn new(
        transform_id: Arc<String>,
        transform: Arc<PTransform>,
        context: Arc<OperatorContext>,
        operator_discriminant: OperatorDiscriminants,
    ) -> Self
    where
        Self: Sized;

    fn start_bundle(&self);

    fn process(&self, value: &WindowedValue);

    fn finish_bundle(&self) {
        todo!()
    }
}

#[derive(fmt::Debug, EnumDiscriminants)]
pub enum Operator {
    // Test operators
    Create(CreateOperator),
    Recording(RecordingOperator),
    _Partitioning,
    GroupByKey(GroupByKeyWithinBundleOperator),
    Impulse(ImpulsePerBundleOperator),

    // Production operators
    _DataSource,
    ParDo(ParDoOperator),
    Flatten(FlattenOperator),
}

impl OperatorI for Operator {
    fn new(
        transform_id: Arc<String>,
        transform: Arc<PTransform>,
        context: Arc<OperatorContext>,
        operator_discriminant: OperatorDiscriminants,
    ) -> Self {
        match operator_discriminant {
            OperatorDiscriminants::Create => Operator::Create(CreateOperator::new(
                transform_id,
                transform,
                context,
                operator_discriminant,
            )),
            _ => todo!(),
        }
    }

    fn start_bundle(&self) {
        match self {
            Operator::Create(create_op) => create_op.start_bundle(),
            Operator::Recording(recording_op) => recording_op.start_bundle(),
            Operator::Impulse(impulse_op) => impulse_op.start_bundle(),
            Operator::GroupByKey(gbk_op) => gbk_op.start_bundle(),
            Operator::ParDo(pardo_op) => pardo_op.start_bundle(),
            Operator::Flatten(flatten_op) => flatten_op.start_bundle(),
            _ => todo!(),
        };
    }

    fn process(&self, value: &WindowedValue) {
        match self {
            Operator::Create(create_op) => {
                create_op.process(value);
            }
            Operator::Recording(recording_op) => {
                recording_op.process(value);
            }
            Operator::Impulse(impulse_op) => {
                impulse_op.process(value);
            }
            Operator::GroupByKey(gbk_op) => {
                gbk_op.process(value);
            }
            Operator::ParDo(pardo_op) => {
                pardo_op.process(value);
            }
            Operator::Flatten(flatten_op) => {
                flatten_op.process(value);
            }
            _ => todo!(),
        };
    }

    fn finish_bundle(&self) {
        match self {
            Operator::Create(create_op) => create_op.finish_bundle(),
            Operator::Recording(recording_op) => recording_op.finish_bundle(),
            Operator::Impulse(impulse_op) => impulse_op.finish_bundle(),
            Operator::GroupByKey(gbk_op) => gbk_op.finish_bundle(),
            Operator::ParDo(pardo_op) => pardo_op.finish_bundle(),
            Operator::Flatten(flatten_op) => flatten_op.finish_bundle(),
            _ => todo!(),
        };
    }
}

pub fn create_operator(transform_id: &str, context: Arc<OperatorContext>) -> Operator {
    let descriptor: &ProcessBundleDescriptor = context.descriptor.as_ref();

    let transform = descriptor
        .transforms
        .get(transform_id)
        .expect("Transform ID not found");

    for pcoll_id in transform.outputs.values() {
        (context.get_receiver)(context.bundle_processor.clone(), pcoll_id.clone());
    }

    let operators_by_urn = OPERATORS_BY_URN.lock().unwrap();

    let spec = transform
        .spec
        .as_ref()
        .unwrap_or_else(|| panic!("Transform {} has no spec", transform_id));

    let op_discriminant = operators_by_urn
        .get(spec.urn.as_str())
        .unwrap_or_else(|| panic!("Unknown transform type: {}", spec.urn));

    match op_discriminant {
        OperatorDiscriminants::Create => Operator::Create(CreateOperator::new(
            Arc::new(transform_id.to_string()),
            Arc::new(transform.clone()),
            context.clone(),
            OperatorDiscriminants::Create,
        )),
        OperatorDiscriminants::Recording => Operator::Recording(RecordingOperator::new(
            Arc::new(transform_id.to_string()),
            Arc::new(transform.clone()),
            context.clone(),
            OperatorDiscriminants::Recording,
        )),
        OperatorDiscriminants::Impulse => Operator::Impulse(ImpulsePerBundleOperator::new(
            Arc::new(transform_id.to_string()),
            Arc::new(transform.clone()),
            context.clone(),
            OperatorDiscriminants::Impulse,
        )),
        OperatorDiscriminants::GroupByKey => {
            Operator::GroupByKey(GroupByKeyWithinBundleOperator::new(
                Arc::new(transform_id.to_string()),
                Arc::new(transform.clone()),
                context.clone(),
                OperatorDiscriminants::GroupByKey,
            ))
        }
        OperatorDiscriminants::ParDo => Operator::ParDo(ParDoOperator::new(
            Arc::new(transform_id.to_string()),
            Arc::new(transform.clone()),
            context.clone(),
            OperatorDiscriminants::ParDo,
        )),
        OperatorDiscriminants::Flatten => Operator::Flatten(FlattenOperator::new(
            Arc::new(transform_id.to_string()),
            Arc::new(transform.clone()),
            context.clone(),
            OperatorDiscriminants::ParDo,
        )),
        _ => todo!(),
    }
}

#[derive(Debug)]
pub struct Receiver {
    operators: Vec<Arc<Operator>>,
}

impl Receiver {
    pub fn new(operators: Vec<Arc<Operator>>) -> Self {
        Receiver { operators }
    }

    pub fn receive(&self, value: &WindowedValue) {
        for op in &self.operators {
            op.process(value);
        }
    }
}

pub struct OperatorContext {
    pub descriptor: Arc<ProcessBundleDescriptor>,
    pub get_receiver: Box<dyn Fn(Arc<BundleProcessor>, String) -> Arc<Receiver> + Send + Sync>,
    // get_data_channel: fn(&str) -> MultiplexingDataChannel,
    // get_bundle_id: String,
    pub bundle_processor: Arc<BundleProcessor>,
}

impl fmt::Debug for OperatorContext {
    fn fmt(&self, o: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        o.debug_struct("OperatorContext")
            .field("descriptor", &self.descriptor)
            .field("bundle_processor", &self.bundle_processor)
            .finish()
    }
}

// ******* Windowed Element Primitives *******

pub trait Window: core::fmt::Debug + Send {}

#[derive(Clone, Debug)]
pub struct GlobalWindow;

impl Window for GlobalWindow {}

#[derive(Debug)]
pub struct WindowedValue {
    windows: Rc<Vec<Box<dyn Window>>>,
    timestamp: std::time::Instant,
    pane_info: Box<[u8]>,
    value: Box<dyn Any>,
}

impl WindowedValue {
    fn in_global_window(value: Box<dyn Any>) -> WindowedValue {
        WindowedValue {
            windows: Rc::new(vec![Box::new(GlobalWindow {})]),
            timestamp: std::time::Instant::now(), // TODO: MinTimestamp
            pane_info: Box::new([]),
            value,
        }
    }

    fn with_value(&self, value: Box<dyn Any>) -> WindowedValue {
        WindowedValue {
            windows: self.windows.clone(),
            timestamp: self.timestamp,
            pane_info: self.pane_info.clone(),
            value,
        }
    }
}

// ******* Test Operator definitions *******

#[derive(Debug)]
pub struct CreateOperator {
    _transform_id: Arc<String>,
    _transform: Arc<PTransform>,
    _context: Arc<OperatorContext>,
    _operator_discriminant: OperatorDiscriminants,

    receivers: Vec<Arc<Receiver>>,
    data: Vec<String>,
}

impl OperatorI for CreateOperator {
    fn new(
        transform_id: Arc<String>,
        transform: Arc<PTransform>,
        context: Arc<OperatorContext>,
        operator_discriminant: OperatorDiscriminants,
    ) -> Self {
        let payload = transform
            .as_ref()
            .spec
            .as_ref()
            .expect("No spec found for transform")
            .payload
            .clone();

        let data: Vec<String> = serde_json::from_slice(&payload).unwrap();

        let receivers = transform
            .outputs
            .values()
            .map(|pcollection_id: &String| {
                let bp = context.bundle_processor.clone();
                (context.get_receiver)(bp, pcollection_id.clone())
            })
            .collect();

        Self {
            _transform_id: transform_id,
            _transform: transform,
            _context: context,
            _operator_discriminant: operator_discriminant,
            receivers,
            data,
        }
    }

    fn start_bundle(&self) {
        for datum in &self.data {
            let wv = WindowedValue::in_global_window(Box::new(datum.clone()));
            for rec in self.receivers.iter() {
                rec.receive(&wv);
            }
        }
    }

    fn process(&self, _value: &WindowedValue) {}

    fn finish_bundle(&self) {}
}

#[derive(Debug)]
pub struct RecordingOperator {
    transform_id: Arc<String>,
    _transform: Arc<PTransform>,
    _context: Arc<OperatorContext>,
    _operator_discriminant: OperatorDiscriminants,

    receivers: Vec<Arc<Receiver>>,
}

impl OperatorI for RecordingOperator {
    fn new(
        transform_id: Arc<String>,
        transform: Arc<PTransform>,
        context: Arc<OperatorContext>,
        operator_discriminant: OperatorDiscriminants,
    ) -> Self {
        let receivers = transform
            .outputs
            .values()
            .map(|pcollection_id: &String| {
                let bp = context.bundle_processor.clone();
                (context.get_receiver)(bp, pcollection_id.clone())
            })
            .collect();

        Self {
            transform_id,
            _transform: transform,
            _context: context,
            _operator_discriminant: operator_discriminant,
            receivers,
        }
    }

    fn start_bundle(&self) {
        let mut log = RECORDING_OPERATOR_LOGS.lock().unwrap();
        log.push(format!("{}.start_bundle()", self.transform_id));
    }

    fn process(&self, value: &WindowedValue) {
        unsafe {
            let mut log = RECORDING_OPERATOR_LOGS.lock().unwrap();
            log.push(format!(
                "{}.process({:?})",
                self.transform_id,
                value
                    .value
                    .downcast_ref::<String>()
                    .unwrap_or(&"{any}".to_string())
            ));
        }

        for rec in self.receivers.iter() {
            rec.receive(value);
        }
    }

    fn finish_bundle(&self) {
        let mut log = RECORDING_OPERATOR_LOGS.lock().unwrap();
        log.push(format!("{}.finish_bundle()", self.transform_id));
    }
}

#[derive(Debug)]
pub struct ImpulsePerBundleOperator {
    receivers: Vec<Arc<Receiver>>,
}

impl OperatorI for ImpulsePerBundleOperator {
    fn new(
        _transform_id: Arc<String>,
        transform: Arc<PTransform>,
        context: Arc<OperatorContext>,
        _operator_discriminant: OperatorDiscriminants,
    ) -> Self {
        let receivers = transform
            .outputs
            .values()
            .map(|pcollection_id: &String| {
                let bp = context.bundle_processor.clone();
                (context.get_receiver)(bp, pcollection_id.clone())
            })
            .collect();

        Self { receivers }
    }

    fn start_bundle(&self) {
        let wv = WindowedValue::in_global_window(Box::new(Vec::<u8>::new()));
        for rec in self.receivers.iter() {
            rec.receive(&wv);
        }
    }

    fn process(&self, _value: &WindowedValue) {}

    fn finish_bundle(&self) {}
}

struct GroupByKeyWithinBundleOperator {
    receivers: Vec<Arc<Receiver>>,
    key_extractor: &'static Box<dyn serialize::KeyExtractor>,
    // TODO: Operator requiring locking for structures only ever manipulated in
    // a single thread seems inefficient and overkill.
    grouped_values: Arc<Mutex<HashMap<String, Box<Vec<Box<dyn Any + Send + Sync>>>>>>,
}

impl OperatorI for GroupByKeyWithinBundleOperator {
    fn new(
        _transform_id: Arc<String>,
        transform_proto: Arc<PTransform>,
        context: Arc<OperatorContext>,
        _operator_discriminant: OperatorDiscriminants,
    ) -> Self {
        // TODO: Shared by all operators, move up?
        let receivers = transform_proto
            .outputs
            .values()
            .map(|pcollection_id: &String| {
                let bp = context.bundle_processor.clone();
                (context.get_receiver)(bp, pcollection_id.clone())
            })
            .collect();

        let key_extractor = serialize::deserialize_fn::<Box<dyn serialize::KeyExtractor>>(
            &String::from_utf8(transform_proto.spec.as_ref().unwrap().payload.clone()).unwrap(),
        )
        .unwrap();

        Self {
            receivers,
            key_extractor,
            grouped_values: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn start_bundle(&self) {
        self.grouped_values.lock().unwrap().clear();
    }

    fn process(&self, element: &WindowedValue) {
        // TODO: assumes global window
        let untyped_value: &dyn Any = &*element.value;
        let (key, value) = self.key_extractor.extract(untyped_value);
        let mut grouped_values = self.grouped_values.lock().unwrap();
        if !grouped_values.contains_key(&key) {
            grouped_values.insert(key.clone(), Box::default());
        }
        grouped_values.get_mut(&key).unwrap().push(value);
    }

    fn finish_bundle(&self) {
        for (key, values) in self.grouped_values.lock().unwrap().iter() {
            // TODO: timestamp and pane info are wrong
            for receiver in self.receivers.iter() {
                // TODO: End-of-window timestamp, only firing pane.
                receiver.receive(&WindowedValue::in_global_window(
                    self.key_extractor.recombine(key, values),
                ));
            }
        }
    }
}

impl std::fmt::Debug for GroupByKeyWithinBundleOperator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "GroupByKeyWithinBundleOperator")
    }
}

// ******* Production Operator definitions *******

pub struct ParDoOperator {
    _transform_id: Arc<String>,
    _transform: Arc<PTransform>,
    _context: Arc<OperatorContext>,
    _operator_discriminant: OperatorDiscriminants,

    receivers: Vec<Arc<Receiver>>,
    dofn: &'static serialize::GenericDoFn,
}

impl OperatorI for ParDoOperator {
    fn new(
        transform_id: Arc<String>,
        transform_proto: Arc<PTransform>,
        context: Arc<OperatorContext>,
        operator_discriminant: OperatorDiscriminants,
    ) -> Self {
        // TODO: Shared by all operators, move up?
        let receivers = transform_proto
            .outputs
            .values()
            .map(|pcollection_id: &String| {
                let bp = context.bundle_processor.clone();
                (context.get_receiver)(bp, pcollection_id.clone())
            })
            .collect();

        let dofn = serialize::deserialize_fn::<serialize::GenericDoFn>(
            &String::from_utf8(transform_proto.spec.as_ref().unwrap().payload.clone()).unwrap(),
        )
        .unwrap();

        Self {
            _transform_id: transform_id,
            _transform: transform_proto,
            _context: context,
            _operator_discriminant: operator_discriminant,
            receivers,
            dofn,
        }
    }

    fn start_bundle(&self) {}

    fn process(&self, windowed_element: &WindowedValue) {
        let func = self.dofn; // Can't use self.func directly in a call.
        for output in &mut func(&*windowed_element.value) {
            let windowed_output = windowed_element.with_value(output);
            for rec in self.receivers.iter() {
                rec.receive(&windowed_output);
            }
        }
    }

    fn finish_bundle(&self) {}
}

impl std::fmt::Debug for ParDoOperator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "ParDoOperator")
    }
}

#[derive(Debug)]
pub struct FlattenOperator {
    receivers: Vec<Arc<Receiver>>,
}

impl OperatorI for FlattenOperator {
    fn new(
        _transform_id: Arc<String>,
        transform: Arc<PTransform>,
        context: Arc<OperatorContext>,
        _operator_discriminant: OperatorDiscriminants,
    ) -> Self {
        let receivers = transform
            .outputs
            .values()
            .map(|pcollection_id: &String| {
                let bp = context.bundle_processor.clone();
                (context.get_receiver)(bp, pcollection_id.clone())
            })
            .collect();

        Self { receivers }
    }

    fn start_bundle(&self) {}

    fn process(&self, value: &WindowedValue) {
        for rec in self.receivers.iter() {
            rec.receive(value);
        }
    }

    fn finish_bundle(&self) {}
}
