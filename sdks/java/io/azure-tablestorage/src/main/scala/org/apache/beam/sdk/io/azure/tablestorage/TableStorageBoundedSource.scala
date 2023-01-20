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
package org.apache.beam.sdk.io.azure.tablestorage

import com.azure.data.tables.models.TableEntity
import org.apache.beam.sdk.annotations.Experimental
import org.apache.beam.sdk.annotations.Experimental.Kind
import org.apache.beam.sdk.coders.Coder
import org.apache.beam.sdk.extensions.kryo.KryoCoder
import org.apache.beam.sdk.io.BoundedSource
import org.apache.beam.sdk.options.PipelineOptions
import org.slf4j.LoggerFactory

import java.util
import java.util.Collections

@Experimental(Kind.SOURCE_SINK)
private class TableStorageBoundedSource(val tableStorageRead: TableStorageRead) extends BoundedSource[TableEntity] {
  private val log = LoggerFactory.getLogger("TableStorageBoundedSource")
  log.info("Validating the tableStorageRead")
  tableStorageRead.validate()

  /** @inheritDoc
    *   TODO: You have to find a better way, maybe by partition key
    */
  override def split(desiredBundleSizeBytes: Long, options: PipelineOptions): util.List[TableStorageBoundedSource] =
    Collections.singletonList(this)

  /** @inheritDoc
    *   The Cosmos DB Coro (SQL) API not support this metrics by the querys
    */
  override def getEstimatedSizeBytes(options: PipelineOptions): Long = 0L

  override def getOutputCoder: Coder[TableEntity] = KryoCoder.of[TableEntity]()

  override def createReader(options: PipelineOptions): BoundedSource.BoundedReader[TableEntity] = {
    log.info("Creating TableStorageBoundedReader")
    new TableStorageBoundedReader(this)
  }
}
