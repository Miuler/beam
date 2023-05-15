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

use std::sync::Arc;

use crate::elem_types::ElemType;
use crate::internals::pipeline::Pipeline;
use crate::internals::pvalue::{PTransform, PValue};
use crate::internals::urns::FLATTEN_URN;
use crate::proto::pipeline::v1 as pipeline_v1;

pub struct Flatten {}

impl Flatten {
    pub fn new() -> Self {
        Self {}
    }
}

// TODO: The type signature should indicate only PCollection arrays are accepted.
impl<In, Out> PTransform<In, Out> for Flatten
where
    In: ElemType,
    Out: ElemType + Clone,
{
    fn expand_internal(
        &self,
        _input: &PValue<In>,
        pipeline: Arc<Pipeline>,
        transform_proto: &mut pipeline_v1::PTransform,
    ) -> PValue<Out> {
        let spec = pipeline_v1::FunctionSpec {
            urn: FLATTEN_URN.to_string(),
            payload: crate::internals::urns::IMPULSE_BUFFER.to_vec(), // Should be able to omit.
        };
        transform_proto.spec = Some(spec);

        // TODO: add coder id
        pipeline.create_pcollection_internal("".to_string(), pipeline.clone())
    }
}

impl Default for Flatten {
    fn default() -> Self {
        Self::new()
    }
}