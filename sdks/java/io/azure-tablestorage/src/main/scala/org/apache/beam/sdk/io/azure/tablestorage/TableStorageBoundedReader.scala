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

import com.azure.data.tables.models.{ ListEntitiesOptions, TableEntity }
import com.azure.data.tables.{ TableServiceAsyncClient, TableServiceClientBuilder }
import org.apache.beam.sdk.annotations.Experimental
import org.apache.beam.sdk.annotations.Experimental.Kind
import org.apache.beam.sdk.io.BoundedSource
import org.slf4j.LoggerFactory

@Experimental(Kind.SOURCE_SINK)
private class TableStorageBoundedReader(tableStorageBoundedSource: TableStorageBoundedSource)
    extends BoundedSource.BoundedReader[TableEntity] {
  private val log = LoggerFactory.getLogger(getClass)
  private var maybeTableServiceAsyncClient: Option[TableServiceAsyncClient] = None
  private var maybeIterator: Option[java.util.Iterator[TableEntity]] = None

  override def start(): Boolean = {
    log.debug("TableStorageBoundedReader.start()")

    maybeTableServiceAsyncClient = Some(
      new TableServiceClientBuilder()
        .endpoint(tableStorageBoundedSource.tableStorageRead.endpoint)
        .sasToken(tableStorageBoundedSource.tableStorageRead.sasToken)
        .buildAsyncClient()
    )

    maybeIterator = maybeTableServiceAsyncClient.map { tableServiceAsyncClient =>
      log.info("Get the container name")
      log.info(s"Get the iterator of the query in container ${tableStorageBoundedSource.tableStorageRead.tableName}")

      val listEntitiesOptions = new ListEntitiesOptions

      val query = tableStorageBoundedSource.tableStorageRead.query
      if (query != null && !query.isBlank) {
        listEntitiesOptions.setFilter(query)
      }

      tableServiceAsyncClient
        .getTableClient(tableStorageBoundedSource.tableStorageRead.tableName)
        .listEntities(listEntitiesOptions)
        .toIterable
        .iterator()
    }

    maybeIterator.isDefined
  }

  override def advance(): Boolean = maybeIterator.exists(_.hasNext)

  override def getCurrent: TableEntity = maybeIterator
    .filter(_.hasNext)
    .map(_.next())
    .orNull

  override def getCurrentSource: TableStorageBoundedSource = tableStorageBoundedSource

  override def close(): Unit = ()
}
