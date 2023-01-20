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
import edu.umd.cs.findbugs.annotations.SuppressFBWarnings
import org.apache.beam.sdk.annotations.Experimental
import org.apache.beam.sdk.annotations.Experimental.Kind
import org.apache.beam.sdk.io.Read
import org.apache.beam.sdk.transforms.PTransform
import org.apache.beam.sdk.values.{ PBegin, PCollection }
import org.slf4j.LoggerFactory

@Experimental(Kind.SOURCE_SINK)
@SuppressFBWarnings(Array("MS_PKGPROTECT"))
case class TableStorageRead(
    private[tablestorage] val endpoint: String = null,
    private[tablestorage] val sasToken: String = null,
    private[tablestorage] val tableName: String = null,
    private[tablestorage] val query: String = null
) extends PTransform[PBegin, PCollection[TableEntity]] {

  private val log = LoggerFactory.getLogger(classOf[TableStorageRead])

  /** Create new ReadCosmos based into previous ReadCosmos, modifying the endpoint */
  def withEndpoint(endpoint: String): TableStorageRead = this.copy(endpoint = endpoint)

  def withSasToken(key: String): TableStorageRead = this.copy(sasToken = key)

  def withQuery(query: String): TableStorageRead = this.copy(query = query)

  def withTableName(tableName: String): TableStorageRead = this.copy(tableName = tableName)

  override def expand(input: PBegin): PCollection[TableEntity] = {
    log.info(s"Read CosmosDB with endpoint: $endpoint and query: $query")

    input.apply(Read.from(new TableStorageBoundedSource(this)))
  }

  def validate(): Unit = {
    require(endpoint != null && !endpoint.isBlank, "CosmosDB endpoint is required")
    require(sasToken != null && !sasToken.isBlank, "CosmosDB key is required")
    require(tableName != null && !tableName.isBlank, "CosmosDB container is required")
  }
}
